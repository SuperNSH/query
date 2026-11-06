use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_query::{
    CachePolicy, InfiniteQueryResource, QueryError, QueryKey, QueryStatus, RequestId,
    RequestPolicy, RetryPolicy,
};
use nsh_core::prelude::*;

use crate::now_ms;
use crate::options::QueryContext;
use crate::tracked::Tracked;

type PageFetcher<P, T, E> =
    Rc<dyn Fn(P, QueryContext) -> Pin<Box<dyn Future<Output = Result<T, E>>>>>;
type NextParam<P, T> = Rc<dyn Fn(&T) -> Option<P>>;

/// How an infinite query pages. `get_next` reads the next page's param from a
/// page, and `None` means that page was the last.
pub struct InfiniteQueryOptions<P: 'static, T: 'static> {
    initial: P,
    get_next: NextParam<P, T>,
    get_previous: Option<NextParam<P, T>>,
    max_pages: Option<usize>,
    retry: RetryPolicy,
}

impl<P, T> InfiniteQueryOptions<P, T> {
    pub fn new(initial: P, get_next: impl Fn(&T) -> Option<P> + 'static) -> Self {
        Self {
            initial,
            get_next: Rc::new(get_next),
            get_previous: None,
            max_pages: None,
            retry: RetryPolicy::default(),
        }
    }

    /// Enables `fetch_previous_page`, for a list opened in the middle of its data.
    pub fn get_previous(mut self, f: impl Fn(&T) -> Option<P> + 'static) -> Self {
        self.get_previous = Some(Rc::new(f));
        self
    }

    /// Keeps at most this many pages, dropping from the far end.
    pub fn max_pages(mut self, max: usize) -> Self {
        self.max_pages = Some(max);
        self
    }

    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }
}

struct Shared<P: 'static, T: 'static, E: 'static> {
    fetcher: PageFetcher<P, T, E>,
    initial: P,
    get_next: NextParam<P, T>,
    get_previous: Option<NextParam<P, T>>,
    next: Option<P>,
    previous: Option<P>,
    retry: RetryPolicy,
}

/// Pages of a list a component observes. `Copy`; reads are tracked. Owned by
/// the component, and a component that goes takes its pages with it.
pub struct InfiniteQuery<P: 'static, T: 'static, E: 'static = QueryError> {
    resource: Tracked<InfiniteQueryResource<T, E>>,
    shared: Stored<Shared<P, T, E>>,
}

impl<P, T, E> Clone for InfiniteQuery<P, T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, T, E> Copy for InfiniteQuery<P, T, E> {}

impl<P: Clone + 'static, T: 'static, E: 'static> InfiniteQuery<P, T, E> {
    /// The loaded pages in order. Pages are `Arc`s, so this copies no page data.
    pub fn pages(&self) -> Vec<Arc<T>> {
        self.resource.with(|r| r.pages().iter().cloned().collect())
    }

    pub fn page_count(&self) -> usize {
        self.resource.with(InfiniteQueryResource::page_count)
    }

    pub fn has_next_page(&self) -> bool {
        self.resource.with(InfiniteQueryResource::has_next_page)
    }

    pub fn has_previous_page(&self) -> bool {
        self.resource.with(InfiniteQueryResource::has_previous_page)
    }

    pub fn is_fetching_next_page(&self) -> bool {
        self.resource
            .with(InfiniteQueryResource::is_fetching_next_page)
    }

    pub fn is_fetching_previous_page(&self) -> bool {
        self.resource
            .with(InfiniteQueryResource::is_fetching_previous_page)
    }

    pub fn status(&self) -> QueryStatus {
        self.resource.with(InfiniteQueryResource::status)
    }

    /// True until the first page arrives.
    pub fn is_pending(&self) -> bool {
        self.resource.with(|r| {
            r.page_count() == 0
                && matches!(r.status(), QueryStatus::Idle | QueryStatus::LoadingEmpty)
        })
    }

    pub fn error(&self) -> Option<E>
    where
        E: Clone,
    {
        self.resource.with(|r| r.error().cloned())
    }

    /// Loads the next page. Does nothing past the last page, or while one is
    /// on its way, so it is safe to call from `on_load_more`.
    pub fn fetch_next_page(&self) {
        let Some(param) = self.shared.with(|s| s.next.clone()) else {
            return;
        };
        let now = now_ms();
        let mut began = None;
        self.resource
            .update(|r| began = r.begin_fetch_next_with_id(None, now));
        if let Some(request_id) = began {
            self.run(request_id, param, true);
        }
    }

    /// Loads the page before the first. Needs `InfiniteQueryOptions::get_previous`.
    pub fn fetch_previous_page(&self) {
        let Some(param) = self.shared.with(|s| s.previous.clone()) else {
            return;
        };
        let now = now_ms();
        let mut began = None;
        self.resource
            .update(|r| began = r.begin_fetch_previous_with_id(None, now));
        if let Some(request_id) = began {
            self.run(request_id, param, false);
        }
    }

    /// Drops the pages and loads from the first page again.
    pub fn refetch(&self) {
        self.resource.update(InfiniteQueryResource::reset);
        self.shared.with(|s| {
            s.next = Some(s.initial.clone());
            s.previous = None;
        });
        self.resource.update(|r| r.set_has_next_page(true));
        self.fetch_next_page();
    }

    fn run(&self, request_id: RequestId, param: P, is_next: bool) {
        let query = *self;
        spawn(async move {
            let (fetcher, retry) = query.shared.with(|s| (s.fetcher.clone(), s.retry.clone()));
            let mut attempt = 0;
            loop {
                let (key, signal) = query
                    .resource
                    .with_untracked(|r| (r.key().clone(), r.signal().cloned().unwrap_or_default()));
                let result = fetcher(param.clone(), QueryContext { key, signal }).await;
                if !query
                    .resource
                    .with_untracked(|r| r.is_current_request(request_id))
                {
                    return;
                }
                match result {
                    Ok(page) => {
                        let (next, previous) = query.shared.with(|s| {
                            (
                                (s.get_next)(&page),
                                s.get_previous.as_ref().and_then(|f| f(&page)),
                            )
                        });
                        let has_more = if is_next {
                            next.is_some()
                        } else {
                            previous.is_some()
                        };
                        let first_page = query.resource.with_untracked(|r| r.page_count() == 0);
                        query.shared.with(|s| {
                            if is_next {
                                s.next = next;
                            }
                            if !is_next || first_page {
                                s.previous = previous.clone();
                            }
                        });
                        let now = now_ms();
                        query.resource.update(|r| {
                            r.reset_retry_count();
                            r.complete_page_success(request_id, page, has_more, is_next, now);
                            if is_next && first_page {
                                r.set_has_previous_page(previous.is_some());
                            }
                        });
                        return;
                    }
                    Err(_) if retry.should_retry(attempt) => {
                        let delay = retry.delay_for_attempt(attempt);
                        attempt += 1;
                        query
                            .resource
                            .update(InfiniteQueryResource::increment_retry);
                        timer(Duration::from_millis(delay)).await;
                    }
                    Err(error) => {
                        query.resource.update(|r| {
                            r.complete_page_failure(request_id, error);
                        });
                        return;
                    }
                }
            }
        });
    }
}

/// Observes a paged list and loads its first page. `fetch_page` gets the page's
/// param and a [`QueryContext`].
pub fn use_infinite_query<P, T, E, F, Fut>(
    key: impl Into<QueryKey>,
    options: InfiniteQueryOptions<P, T>,
    fetch_page: F,
) -> InfiniteQuery<P, T, E>
where
    P: Clone + 'static,
    T: 'static,
    E: 'static,
    F: Fn(P, QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    let mut resource = InfiniteQueryResource::<T, E>::new(
        key.into(),
        CachePolicy::NoCache,
        RequestPolicy::IgnoreWhileLoading,
    );
    resource.set_retry_policy(options.retry.clone());
    if options.max_pages.is_some() {
        let _ = resource.set_max_pages(options.max_pages);
    }
    let fetcher: PageFetcher<P, T, E> =
        Rc::new(move |param, context| Box::pin(fetch_page(param, context)));
    let query = InfiniteQuery {
        resource: Tracked::new(resource),
        shared: stored(Shared {
            fetcher,
            next: Some(options.initial.clone()),
            previous: None,
            initial: options.initial,
            get_next: options.get_next,
            get_previous: options.get_previous,
            retry: options.retry,
        }),
    };
    query.fetch_next_page();
    query
}
