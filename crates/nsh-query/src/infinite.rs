use std::any::{Any, TypeId};
use std::future::{Future, poll_fn};
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Poll, Waker};
use std::time::Duration;

use gpui_query::{
    CachePolicy, InfiniteQueryResource, QueryError, QueryKey, QueryStatus, RequestId,
    RequestPolicy, RetryPolicy,
};
use nsh_core::JsonValue;
use nsh_core::element::create_root;
use nsh_core::prelude::*;

use crate::client::{AnyEntry, QueryClient, QueryFilters, QueryState, query_client};
use crate::diagnostics::QueryDiagnostic;
use crate::now_ms;
use crate::options::{PageDirection, QueryContext};
use crate::query::{FetchQueryError, Meta};
use crate::status::{FetchStatus, Outcome};
use crate::tracked::Tracked;

type PageFetcher<P, T, E> =
    Rc<dyn Fn(P, QueryContext) -> Pin<Box<dyn Future<Output = Result<T, E>>>>>;
type NextParam<P, T> = Rc<dyn Fn(&T) -> Option<P>>;

static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// How an infinite query pages. `get_next` reads the next page's param from a
/// page, and `None` means that page was the last.
pub struct InfiniteQueryOptions<P: 'static, T: 'static> {
    meta: Option<JsonValue>,
    initial: P,
    get_next: NextParam<P, T>,
    get_previous: Option<NextParam<P, T>>,
    max_pages: Option<usize>,
    retry: RetryPolicy,
}

impl<P, T> InfiniteQueryOptions<P, T> {
    pub fn new(initial: P, get_next: impl Fn(&T) -> Option<P> + 'static) -> Self {
        Self {
            meta: None,
            initial,
            get_next: Rc::new(get_next),
            get_previous: None,
            max_pages: None,
            retry: RetryPolicy::default(),
        }
    }

    /// TanStack's `meta`: passed to every page fetch as `QueryContext::meta`.
    pub fn meta(mut self, meta: JsonValue) -> Self {
        self.meta = Some(meta);
        self
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

    /// No page limit: every loaded page stays. This is the default, as in TanStack, and it
    /// undoes an earlier [`Self::max_pages`].
    pub fn unbounded_pages(mut self) -> Self {
        self.max_pages = None;
        self
    }

    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }
}

/// Which end a page load adds to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Next,
    Previous,
}

/// The params that produced the loaded pages, and where the next and
/// previous pages start.
struct Pages<P> {
    initial: P,
    next: Option<P>,
    previous: Option<P>,
    /// One param per loaded page, first page first.
    params: Vec<P>,
    error_next: bool,
    error_previous: bool,
    /// Bumped on every change to this struct, so a memo over it can follow it.
    epoch: u64,
}

/// The memos over an infinite entry's cells, one per value a public read shows.
#[derive(Clone, Copy)]
struct IFields {
    pages_epoch: Memo<u64>,
    page_count: Memo<usize>,
    params_epoch: Memo<u64>,
    has_next: Memo<bool>,
    has_previous: Memo<bool>,
    fetching_next: Memo<bool>,
    fetching_previous: Memo<bool>,
    core_status: Memo<QueryStatus>,
    error_key: Memo<(bool, u32)>,
    data_at: Memo<u64>,
}

impl IFields {
    fn new<P: 'static, T: 'static, E: 'static>(
        resource: Tracked<InfiniteQueryResource<T, E>>,
        meta: Tracked<Meta<E>>,
        pages: Tracked<Pages<P>>,
    ) -> Self {
        Self {
            pages_epoch: computed(move || resource.with(|r| r.data_epoch())),
            page_count: computed(move || resource.with(|r| r.page_count())),
            params_epoch: computed(move || pages.with(|p| p.epoch)),
            has_next: computed(move || resource.with(|r| r.has_next_page())),
            has_previous: computed(move || resource.with(|r| r.has_previous_page())),
            fetching_next: computed(move || resource.with(|r| r.is_fetching_next_page())),
            fetching_previous: computed(move || resource.with(|r| r.is_fetching_previous_page())),
            core_status: computed(move || resource.with(|r| r.status())),
            error_key: computed(move || {
                (
                    resource.with(|r| r.error().is_some()),
                    meta.with(|m| m.error_update_count),
                )
            }),
            data_at: computed(move || meta.with(|m| m.data_updated_at)),
        }
    }
}

struct Shared<P: 'static, T: 'static, E: 'static> {
    meta: Option<JsonValue>,
    /// `None` until an observer or prefetch supplies one; loads do nothing before that.
    fetcher: Option<PageFetcher<P, T, E>>,
    get_next: NextParam<P, T>,
    get_previous: Option<NextParam<P, T>>,
    retry: RetryPolicy,
    gc: Duration,
    observers: usize,
    gc_timer: Option<Spawned<()>>,
    running: Option<Spawned<()>>,
    waiters: Vec<Waker>,
}

/// Marks the cache slot of one infinite query type, so it cannot collide with a query.
struct Infinite<P, T, E>(PhantomData<(P, T, E)>);

/// One cached infinite query: its pages, under an owner of its own.
pub(crate) struct InfEntry<P: 'static, T: 'static, E: 'static> {
    uid: u64,
    resource: Tracked<InfiniteQueryResource<T, E>>,
    meta: Tracked<Meta<E>>,
    pages: Tracked<Pages<P>>,
    fields: IFields,
    shared: Stored<Shared<P, T, E>>,
    owner: Owner,
}

impl<P, T, E> Clone for InfEntry<P, T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, T, E> Copy for InfEntry<P, T, E> {}

impl<P: Clone + 'static, T: 'static, E: 'static> AnyEntry for InfEntry<P, T, E> {
    fn uid(&self) -> u64 {
        self.uid
    }

    fn state(&self) -> QueryState {
        let (outcome, fetching, paused, invalidated, data_at, error_at, data_n, error_n, failures) =
            self.meta.with_untracked(|m| {
                (
                    m.outcome,
                    m.fetching,
                    m.paused,
                    m.invalidated,
                    m.data_updated_at,
                    m.error_updated_at,
                    m.data_update_count,
                    m.error_update_count,
                    m.failure_count,
                )
            });
        let has_data = self.resource.with_untracked(|r| r.page_count() > 0);
        QueryState {
            key: self.resource.with_untracked(|r| r.key().clone()),
            status: outcome.status(),
            fetch_status: match (fetching, paused) {
                (true, true) => FetchStatus::Paused,
                (true, false) => FetchStatus::Fetching,
                _ => FetchStatus::Idle,
            },
            data_updated_at: data_at,
            error_updated_at: error_at,
            data_update_count: data_n,
            error_update_count: error_n,
            failure_count: failures,
            is_invalidated: invalidated,
            is_stale: invalidated || !has_data,
            observers: self.shared.with(|s| s.observers),
            has_data,
        }
    }

    fn invalidate(&self) {
        self.meta.update(|m| m.invalidated = true);
        query_client().touch();
        if self.shared.with(|s| s.observers) > 0 {
            self.refetch();
        }
    }

    fn cancel(&self) {
        self.cancel_running();
    }

    fn reset(&self) {
        self.reset_pages();
        if self.shared.with(|s| s.observers) > 0 {
            self.fetch_next();
        }
    }

    fn refetch(&self) {
        self.refetch_all();
    }

    fn dispose(&self) {
        self.owner.dispose();
    }

    fn is_held(&self) -> bool {
        self.shared.with(|s| s.observers > 0)
    }

    fn diagnostic(&self, now: u64) -> QueryDiagnostic {
        self.resource.with_untracked(|r| QueryDiagnostic {
            key: r.key().to_path(),
            status: r.status(),
            cache_policy: r.cache_policy().label(),
            cache_age_ms: r.cache_age_ms(now),
            cache_hits: r.cache_hits(),
            retry_count: r.retry_count(),
        })
    }

    fn dehydrate(&self) -> Option<(JsonValue, u64)> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<P: Clone + 'static, T: 'static, E: 'static> InfEntry<P, T, E> {
    fn id(key: &QueryKey) -> (TypeId, QueryKey) {
        (TypeId::of::<Infinite<P, T, E>>(), key.clone())
    }

    fn find(client: &QueryClient, key: &QueryKey) -> Option<Self> {
        client.lookup::<Self>(&Self::id(key))
    }

    fn get_or_create(
        client: &QueryClient,
        key: &QueryKey,
        options: &InfiniteQueryOptions<P, T>,
    ) -> Self {
        if let Some(entry) = Self::find(client, key) {
            entry.configure(options);
            return entry;
        }
        let uid = NEXT_UID.fetch_add(1, Ordering::Relaxed);
        let mut resource = InfiniteQueryResource::<T, E>::new(
            key.clone(),
            CachePolicy::NoCache,
            RequestPolicy::IgnoreWhileLoading,
        );
        resource.set_retry_policy(options.retry.clone());
        // The core defaults to 50 pages, so `None` must be passed on to mean no limit.
        let _ = resource.set_max_pages(options.max_pages);
        let (owner, entry) = create_root(|| {
            let resource = Tracked::new(resource);
            let meta = Tracked::new(Meta::default());
            let pages = Tracked::new(Pages {
                initial: options.initial.clone(),
                next: Some(options.initial.clone()),
                previous: None,
                params: Vec::new(),
                error_next: false,
                error_previous: false,
                epoch: 0,
            });
            InfEntry {
                uid,
                resource,
                meta,
                pages,
                fields: IFields::new(resource, meta, pages),
                shared: stored(Shared {
                    fetcher: None,
                    get_next: options.get_next.clone(),
                    get_previous: options.get_previous.clone(),
                    retry: options.retry.clone(),
                    meta: options.meta.clone(),
                    gc: Duration::from_secs(300),
                    observers: 0,
                    gc_timer: None,
                    running: None,
                    waiters: Vec::new(),
                }),
                owner: Owner::current(),
            }
        });
        let entry = InfEntry { owner, ..entry };
        client.insert(Self::id(key), Rc::new(entry));
        // Nothing observes a new entry yet, so its gc starts now, as for a query.
        entry.settle();
        entry
    }
    /// Changes the pages and bumps their epoch, so memos over them follow.
    fn update_pages(&self, f: impl FnOnce(&mut Pages<P>)) {
        self.pages.update(|p| {
            p.epoch += 1;
            f(p);
        });
    }

    fn configure(&self, options: &InfiniteQueryOptions<P, T>) {
        self.shared.with(|s| {
            s.get_next = options.get_next.clone();
            s.get_previous = options.get_previous.clone();
            s.retry = options.retry.clone();
            s.meta = options.meta.clone();
        });
        if self.resource.with_untracked(|r| r.max_pages()) != options.max_pages {
            self.resource.update(|r| {
                let _ = r.set_max_pages(options.max_pages);
            });
        }
    }

    fn set_fetcher(&self, fetcher: PageFetcher<P, T, E>) {
        self.shared.with(|s| s.fetcher = Some(fetcher));
    }

    fn has_fetcher(&self) -> bool {
        self.shared.with(|s| s.fetcher.is_some())
    }

    fn observe(&self) {
        let timer = self.shared.with(|s| {
            s.observers += 1;
            s.gc_timer.take()
        });
        if let Some(timer) = timer {
            timer.cancel();
        }
        query_client().touch();
    }

    fn release(&self) {
        self.shared
            .with(|s| s.observers = s.observers.saturating_sub(1));
        query_client().touch();
        self.settle();
    }

    /// Starts the gc timer while no observer follows the entry. Once the cache has dropped
    /// it, the owner ends.
    fn settle(&self) {
        if self.shared.with(|s| s.observers) > 0 {
            return;
        }
        if !query_client().is_registered(self.uid) {
            self.owner.dispose();
            return;
        }
        let entry = *self;
        let id = Self::id(&entry.resource.with_untracked(|r| r.key().clone()));
        let uid = self.uid;
        let gc = self.shared.with(|s| s.gc);
        let timer = self.owner.run(|| {
            nsh_core::element::set_timeout(gc, move || {
                query_client().evict_uid(&id, uid);
            })
        });
        let previous = self.shared.with(|s| s.gc_timer.replace(timer));
        if let Some(previous) = previous {
            previous.cancel();
        }
    }

    fn page_count(&self) -> usize {
        self.resource.with_untracked(|r| r.page_count())
    }

    fn is_current(&self, request_id: RequestId) -> bool {
        self.resource
            .with_untracked(|r| r.is_current_request(request_id))
    }

    fn wake_waiters(&self) {
        let waiters = self.shared.with(|s| std::mem::take(&mut s.waiters));
        for waker in waiters {
            waker.wake();
        }
    }

    async fn wait_idle(self) {
        poll_fn(move |cx| {
            if !self.meta.with_untracked(|m| m.fetching) {
                return Poll::Ready(());
            }
            self.shared.with(|s| s.waiters.push(cx.waker().clone()));
            Poll::Pending
        })
        .await
    }

    fn cancel_running(&self) {
        let running = self.shared.with(|s| s.running.take());
        if let Some(task) = running {
            task.cancel();
        }
        self.resource.update(|r| {
            if let Some(signal) = r.signal() {
                signal.cancel();
            }
            if let Some(id) = r.active_request_id() {
                let _ = r.accept_current_request(id);
            }
        });
        self.meta.update(|m| {
            m.fetching = false;
            m.paused = false;
        });
        query_client().touch();
        self.wake_waiters();
    }

    /// Clears the pages and the page params, back to the first page.
    fn reset_pages(&self) {
        self.cancel_running();
        self.resource.update(InfiniteQueryResource::reset);
        self.update_pages(|p| {
            p.next = Some(p.initial.clone());
            p.previous = None;
            p.params.clear();
            p.error_next = false;
            p.error_previous = false;
        });
        self.meta.update(|m| *m = Meta::default());
        query_client().touch();
    }

    /// Loads the page after the last one, if there is one and none is loading.
    fn fetch_next(&self) {
        let Some(param) = self.pages.with_untracked(|p| p.next.clone()) else {
            return;
        };
        if !self.has_fetcher() {
            return;
        }
        let now = now_ms();
        let mut began = None;
        self.resource
            .update(|r| began = r.begin_fetch_next_with_id(None, now));
        if let Some(request_id) = began {
            self.start(request_id, param, Side::Next);
        }
    }

    /// Loads the page before the first one. Needs `InfiniteQueryOptions::get_previous`.
    fn fetch_previous(&self) {
        let Some(param) = self.pages.with_untracked(|p| p.previous.clone()) else {
            return;
        };
        if !self.has_fetcher() {
            return;
        }
        let now = now_ms();
        let mut began = None;
        self.resource
            .update(|r| began = r.begin_fetch_previous_with_id(None, now));
        if let Some(request_id) = began {
            self.start(request_id, param, Side::Previous);
        }
    }

    fn start(&self, request_id: RequestId, param: P, side: Side) {
        self.meta.update(|m| {
            m.fetching = true;
            m.failure_count = 0;
            m.failure_reason = None;
        });
        query_client().touch();
        let entry = *self;
        let task = self
            .owner
            .run(|| spawn(async move { entry.load_page(request_id, param, side).await }));
        let previous = self.shared.with(|s| s.running.replace(task));
        if let Some(previous) = previous {
            previous.cancel();
        }
    }

    async fn load_page(self, request_id: RequestId, param: P, side: Side) {
        let Some(fetcher) = self.shared.with(|s| s.fetcher.clone()) else {
            return;
        };
        let retry = self.shared.with(|s| s.retry.clone());
        let mut attempt = 0;
        loop {
            let (key, signal) = self
                .resource
                .with_untracked(|r| (r.key().clone(), r.signal().cloned().unwrap_or_default()));
            let result = fetcher(
                param.clone(),
                QueryContext {
                    key,
                    signal,
                    meta: self.shared.with(|s| s.meta.clone()),
                    client: query_client(),
                    direction: Some(match side {
                        Side::Next => PageDirection::Next,
                        Side::Previous => PageDirection::Previous,
                    }),
                },
            )
            .await;
            if !self.is_current(request_id) {
                self.resource
                    .update(InfiniteQueryResource::mark_ignored_result);
                return;
            }
            match result {
                Ok(page) => {
                    self.commit_page(request_id, param, page, side);
                    return;
                }
                Err(_) if retry.should_retry(attempt) => {
                    let delay = retry.delay_for_attempt(attempt);
                    attempt += 1;
                    self.resource.update(InfiniteQueryResource::increment_retry);
                    timer(Duration::from_millis(delay)).await;
                }
                Err(error) => {
                    self.resource.update(|r| {
                        r.complete_page_failure(request_id, error);
                    });
                    self.update_pages(|p| match side {
                        Side::Next => p.error_next = true,
                        Side::Previous => p.error_previous = true,
                    });
                    self.finish(Outcome::Error);
                    return;
                }
            }
        }
    }

    fn commit_page(&self, request_id: RequestId, param: P, page: T, side: Side) {
        let (next, previous) = self.shared.with(|s| {
            (
                (s.get_next)(&page),
                s.get_previous.as_ref().and_then(|f| f(&page)),
            )
        });
        let first_page = self.page_count() == 0;
        let has_more = match side {
            Side::Next => next.is_some(),
            Side::Previous => previous.is_some(),
        };
        let now = now_ms();
        self.resource.update(|r| {
            r.reset_retry_count();
            r.complete_page_success(request_id, page, has_more, side == Side::Next, now);
            if side == Side::Next && first_page {
                r.set_has_previous_page(previous.is_some());
            }
        });
        let count = self.page_count();
        self.update_pages(|p| {
            // The first page also tells where previous pages start.
            let sets_previous = side == Side::Previous || first_page;
            match side {
                Side::Next => {
                    p.next = next;
                    p.params.push(param);
                }
                Side::Previous => p.params.insert(0, param),
            }
            if sets_previous {
                p.previous = previous;
            }
            while p.params.len() > count {
                match side {
                    Side::Next => {
                        p.params.remove(0);
                    }
                    Side::Previous => {
                        p.params.pop();
                    }
                }
            }
            p.error_next = false;
            p.error_previous = false;
        });
        self.finish(Outcome::Success);
    }

    fn finish(&self, outcome: Outcome) {
        let now = now_ms();
        self.meta.update(|m| {
            m.fetching = false;
            m.outcome = outcome;
            if outcome == Outcome::Success {
                m.data_updated_at = now;
                m.data_update_count += 1;
                m.failure_count = 0;
                m.invalidated = false;
            } else {
                m.error_updated_at = now;
                m.error_update_count += 1;
            }
        });
        query_client().touch();
        self.wake_waiters();
    }

    /// Loads every loaded page again, in order, each from its own param.
    fn refetch_all(&self) {
        let params = self.pages.with_untracked(|p| p.params.clone());
        if params.is_empty() {
            self.fetch_next();
            return;
        }
        if !self.has_fetcher() {
            return;
        }
        self.cancel_running();
        let now = now_ms();
        let mut began = None;
        self.resource.update(|r| {
            r.set_has_next_page(true);
            began = r.begin_fetch_next_with_id(None, now);
        });
        let Some(request_id) = began else { return };
        self.meta.update(|m| m.fetching = true);
        query_client().touch();
        let entry = *self;
        let task = self.owner.run(|| {
            spawn(async move {
                entry.refetch_pages(request_id, params).await;
            })
        });
        self.shared.with(|s| s.running = Some(task));
    }

    async fn refetch_pages(self, request_id: RequestId, params: Vec<P>) {
        let Some(fetcher) = self.shared.with(|s| s.fetcher.clone()) else {
            return;
        };
        let (get_next, get_previous) = self
            .shared
            .with(|s| (s.get_next.clone(), s.get_previous.clone()));
        let mut loaded: Vec<(P, T)> = Vec::with_capacity(params.len());
        for param in params {
            let (key, signal) = self
                .resource
                .with_untracked(|r| (r.key().clone(), r.signal().cloned().unwrap_or_default()));
            let result = fetcher(
                param.clone(),
                QueryContext {
                    key,
                    signal,
                    meta: self.shared.with(|s| s.meta.clone()),
                    client: query_client(),
                    direction: Some(PageDirection::Next),
                },
            )
            .await;
            if !self.is_current(request_id) {
                self.resource
                    .update(InfiniteQueryResource::mark_ignored_result);
                return;
            }
            match result {
                Ok(page) => loaded.push((param, page)),
                Err(error) => {
                    self.resource.update(|r| {
                        r.complete_page_failure(request_id, error);
                    });
                    self.finish(Outcome::Error);
                    return;
                }
            }
        }
        let next = loaded.last().and_then(|(_, page)| get_next(page));
        let previous = loaded
            .first()
            .and_then(|(_, page)| get_previous.as_ref().and_then(|f| f(page)));
        let now = now_ms();
        let params: Vec<P> = loaded.iter().map(|(p, _)| p.clone()).collect();
        let mut pages: Vec<T> = loaded.into_iter().map(|(_, page)| page).collect();
        let last = pages.pop();
        self.resource.update(|r| {
            r.reset();
            for page in pages {
                r.append_page(page);
            }
            r.set_has_previous_page(previous.is_some());
            r.set_has_next_page(true);
            // Completing the last page through a fresh request sets the status and the next flag.
            if let Some(last) = last
                && let Some(next_id) = r.begin_fetch_next_with_id(None, now)
            {
                r.complete_page_success(next_id, last, next.is_some(), true, now);
            }
        });
        self.update_pages(|p| {
            p.next = next;
            p.previous = previous;
            p.params = params;
            p.error_next = false;
            p.error_previous = false;
        });
        self.finish(Outcome::Success);
    }
}

impl QueryClient {
    /// Snapshots of every cached infinite query with this page param, value and error type,
    /// as gpui-query's `all_infinite_queries`. Plain queries are not included.
    pub fn all_infinite_queries<P: Clone + 'static, T: 'static, E: 'static>(
        &self,
    ) -> Vec<QueryState> {
        self.matching(&QueryFilters::default())
            .into_iter()
            .filter(|entry| entry.as_any().downcast_ref::<InfEntry<P, T, E>>().is_some())
            .map(|entry| entry.state())
            .collect()
    }
}

/// Pages of a list a component observes. `Copy`; reads are tracked. The pages
/// live in the client cache, so they survive a remount within the gc time.
pub struct InfiniteQuery<P: 'static, T: 'static, E: 'static = QueryError> {
    entry: InfEntry<P, T, E>,
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
        self.entry.fields.pages_epoch.get();
        self.entry
            .resource
            .with_untracked(|r| r.pages().iter().cloned().collect())
    }

    pub fn page_count(&self) -> usize {
        self.entry.fields.page_count.get()
    }

    /// The param each loaded page was fetched with, first page first.
    pub fn page_params(&self) -> Vec<P> {
        self.entry.fields.params_epoch.get();
        self.entry.pages.with_untracked(|p| p.params.clone())
    }

    pub fn has_next_page(&self) -> bool {
        self.entry.fields.has_next.get()
    }

    pub fn has_previous_page(&self) -> bool {
        self.entry.fields.has_previous.get()
    }

    pub fn is_fetching_next_page(&self) -> bool {
        self.entry.fields.fetching_next.get()
    }

    pub fn is_fetching_previous_page(&self) -> bool {
        self.entry.fields.fetching_previous.get()
    }

    /// The last next-page load failed.
    pub fn is_fetch_next_page_error(&self) -> bool {
        self.entry.fields.params_epoch.get();
        self.entry.fields.error_key.get();
        self.entry.pages.with_untracked(|p| p.error_next)
            && self.entry.resource.with_untracked(|r| r.error().is_some())
    }

    /// The last previous-page load failed.
    pub fn is_fetch_previous_page_error(&self) -> bool {
        self.entry.fields.params_epoch.get();
        self.entry.fields.error_key.get();
        self.entry.pages.with_untracked(|p| p.error_previous)
            && self.entry.resource.with_untracked(|r| r.error().is_some())
    }

    /// When the pages last loaded, in ms since the Unix epoch. 0 before any load.
    pub fn data_updated_at(&self) -> u64 {
        self.entry.fields.data_at.get()
    }

    pub fn status(&self) -> QueryStatus {
        self.entry.fields.core_status.get()
    }

    /// True until the first page arrives.
    pub fn is_pending(&self) -> bool {
        let status = self.entry.fields.core_status.get();
        self.entry.fields.page_count.get() == 0
            && matches!(status, QueryStatus::Idle | QueryStatus::LoadingEmpty)
    }

    pub fn error(&self) -> Option<E>
    where
        E: Clone,
    {
        self.entry.fields.error_key.get();
        self.entry.resource.with_untracked(|r| r.error().cloned())
    }

    /// Loads the next page. Does nothing past the last page, or while one is
    /// on its way, so it is safe to call from `on_load_more`.
    pub fn fetch_next_page(&self) {
        self.entry.fetch_next();
    }

    /// Loads the page before the first. Needs `InfiniteQueryOptions::get_previous`.
    pub fn fetch_previous_page(&self) {
        self.entry.fetch_previous();
    }

    /// Loads every loaded page again, in order, each with its own param. The
    /// pages stay on screen until the new ones arrive.
    pub fn refetch(&self) {
        self.entry.refetch_all();
    }

    /// Drops the pages and loads from the first page again. The old behavior of [`Self::refetch`].
    pub fn reset(&self) {
        self.entry.reset_pages();
        self.entry.fetch_next();
    }
}

/// Observes a paged list and loads its first page. `fetch_page` gets the page's
/// param and a [`QueryContext`]. Observers of one key share one set of pages.
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
    let client = query_client();
    let key = key.into();
    let entry = InfEntry::<P, T, E>::get_or_create(&client, &key, &options);
    let fetcher: PageFetcher<P, T, E> =
        Rc::new(move |param, context| Box::pin(fetch_page(param, context)));
    entry.set_fetcher(fetcher);
    entry.observe();
    if entry.page_count() == 0 {
        entry.fetch_next();
    }
    cleanup(move || entry.release());
    InfiniteQuery { entry }
}

impl QueryClient {
    /// Loads the first page of an infinite query into the cache without an observer.
    pub fn prefetch_infinite_query<P, T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: InfiniteQueryOptions<P, T>,
        fetch_page: F,
    ) where
        P: Clone + 'static,
        T: 'static,
        E: 'static,
        F: Fn(P, QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        let key = key.into();
        let entry = InfEntry::<P, T, E>::get_or_create(self, &key, &options);
        let fetcher: PageFetcher<P, T, E> =
            Rc::new(move |param, context| Box::pin(fetch_page(param, context)));
        entry.set_fetcher(fetcher);
        if entry.page_count() == 0 {
            entry.fetch_next();
        }
    }

    /// Returns the cached pages when there are any, else loads the first page.
    pub async fn ensure_infinite_query_data<P, T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: InfiniteQueryOptions<P, T>,
        fetch_page: F,
    ) -> Result<Vec<Arc<T>>, FetchQueryError<E>>
    where
        P: Clone + 'static,
        T: 'static,
        E: Clone + 'static,
        F: Fn(P, QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        let key = key.into();
        let entry = InfEntry::<P, T, E>::get_or_create(self, &key, &options);
        let fetcher: PageFetcher<P, T, E> =
            Rc::new(move |param, context| Box::pin(fetch_page(param, context)));
        entry.set_fetcher(fetcher);
        if entry.page_count() == 0 {
            entry.fetch_next();
        }
        entry.wait_idle().await;
        let pages = entry
            .resource
            .with_untracked(|r| r.pages().iter().cloned().collect::<Vec<_>>());
        if !pages.is_empty() {
            return Ok(pages);
        }
        let error = entry.resource.with_untracked(|r| r.error().cloned());
        Err(match error {
            Some(error) => FetchQueryError::Failed(error),
            None => FetchQueryError::Cancelled,
        })
    }

    /// TanStack's `infiniteQuery(options)`: the same as [`Self::fetch_infinite_query`],
    /// which reloads the pages every time (TanStack's default stale time of zero).
    pub async fn infinite_query<P, T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: InfiniteQueryOptions<P, T>,
        fetch_page: F,
    ) -> Result<Vec<Arc<T>>, FetchQueryError<E>>
    where
        P: Clone + 'static,
        T: 'static,
        E: Clone + 'static,
        F: Fn(P, QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        self.fetch_infinite_query(key, options, fetch_page).await
    }

    /// Reloads the pages every time, unlike [`Self::ensure_infinite_query_data`].
    pub async fn fetch_infinite_query<P, T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: InfiniteQueryOptions<P, T>,
        fetch_page: F,
    ) -> Result<Vec<Arc<T>>, FetchQueryError<E>>
    where
        P: Clone + 'static,
        T: 'static,
        E: Clone + 'static,
        F: Fn(P, QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        let key = key.into();
        let entry = InfEntry::<P, T, E>::get_or_create(self, &key, &options);
        let fetcher: PageFetcher<P, T, E> =
            Rc::new(move |param, context| Box::pin(fetch_page(param, context)));
        entry.set_fetcher(fetcher);
        // TanStack's fetchInfiniteQuery runs fetchQuery with a zero stale time, so it reloads.
        entry.refetch_all();
        entry.wait_idle().await;
        let pages = entry
            .resource
            .with_untracked(|r| r.pages().iter().cloned().collect::<Vec<_>>());
        if !pages.is_empty() {
            return Ok(pages);
        }
        let error = entry.resource.with_untracked(|r| r.error().cloned());
        Err(match error {
            Some(error) => FetchQueryError::Failed(error),
            None => FetchQueryError::Cancelled,
        })
    }
}
