use std::any::{Any, TypeId};
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

use gpui_query::{
    QueryBeginResult, QueryError, QueryFetchMode, QueryKey, QueryResource, QueryStatus,
    RefetchTrigger, RequestId, RequestPolicy, RetryPolicy,
};
use nsh_core::element::{Lifecycle, create_root, lifecycle, set_interval, set_timeout};
use nsh_core::prelude::*;

use crate::client::{AnyEntry, QueryClient, query_client};
use crate::now_ms;
use crate::options::{QueryContext, QueryOptions};
use crate::tracked::Tracked;

pub(crate) type Fetcher<T, E> =
    Rc<dyn Fn(QueryContext) -> Pin<Box<dyn Future<Output = Result<T, E>>>>>;

struct Shared<T: 'static, E: 'static> {
    fetcher: Option<Fetcher<T, E>>,
    retry: RetryPolicy,
    gc: Duration,
    observers: usize,
    gc_timer: Option<Spawned<()>>,
    running: Option<Spawned<()>>,
}

/// One cached query: its state machine as a signal, under an owner of its own
/// so the entry outlives the components observing it.
pub(crate) struct Entry<T: 'static, E: 'static> {
    resource: Tracked<QueryResource<T, E>>,
    shared: Stored<Shared<T, E>>,
    owner: Owner,
}

impl<T, E> Clone for Entry<T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, E> Copy for Entry<T, E> {}

impl<T: 'static, E: 'static> AnyEntry for Entry<T, E> {
    fn key(&self) -> QueryKey {
        self.resource.with_untracked(|r| r.key().clone())
    }

    fn invalidate(&self) {
        self.resource.update(QueryResource::invalidate);
        if self.shared.with(|s| s.observers) > 0 {
            self.reload();
        }
    }

    fn cancel(&self) {
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
    }

    fn dispose(&self) {
        self.owner.dispose();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: 'static, E: 'static> Entry<T, E> {
    fn id(key: &QueryKey) -> (TypeId, QueryKey) {
        (TypeId::of::<(T, E)>(), key.clone())
    }

    pub(crate) fn find(client: &QueryClient, key: &QueryKey) -> Option<Self> {
        client.lookup::<Self>(&Self::id(key))
    }

    pub(crate) fn get_or_create(
        client: &QueryClient,
        key: &QueryKey,
        options: &QueryOptions,
    ) -> Self {
        if let Some(entry) = Self::find(client, key) {
            return entry;
        }
        let (owner, entry) = create_root(|| {
            let mut resource =
                QueryResource::<T, E>::new(key.clone(), options.cache, options.request);
            resource.set_retry_policy(options.retry.clone());
            Entry {
                resource: Tracked::new(resource),
                shared: stored(Shared {
                    fetcher: None,
                    retry: options.retry.clone(),
                    gc: options.gc,
                    observers: 0,
                    gc_timer: None,
                    running: None,
                }),
                owner: Owner::current(),
            }
        });
        let entry = Entry { owner, ..entry };
        client.insert(Self::id(key), Rc::new(entry));
        entry
    }

    pub(crate) fn set_fetcher(&self, fetcher: Fetcher<T, E>) {
        self.shared.with(|s| s.fetcher = Some(fetcher));
    }

    pub(crate) fn data_untracked(&self) -> Option<T>
    where
        T: Clone,
    {
        self.resource.with_untracked(|r| r.data().cloned())
    }

    /// Commits `data` as a finished load, replacing any load in flight.
    pub(crate) fn set_fresh(&self, data: T) {
        let now = now_ms();
        self.resource.update(|r| {
            let policy = r.request_policy();
            r.set_request_policy(RequestPolicy::LatestWins);
            if let QueryBeginResult::Started { request_id, .. } =
                r.begin_request_with_id(None, now, QueryFetchMode::Force)
            {
                r.complete_current_success(request_id, data, now);
            }
            r.set_request_policy(policy);
        });
    }

    fn observe(&self) {
        let timer = self.shared.with(|s| {
            s.observers += 1;
            s.gc_timer.take()
        });
        if let Some(timer) = timer {
            timer.cancel();
        }
    }

    fn release(&self) {
        let (observers, gc) = self.shared.with(|s| {
            s.observers = s.observers.saturating_sub(1);
            (s.observers, s.gc)
        });
        if observers > 0 {
            return;
        }
        let entry = *self;
        let timer = self.owner.run(|| {
            set_timeout(gc, move || {
                query_client().evict(&Self::id(&AnyEntry::key(&entry)));
            })
        });
        self.shared.with(|s| s.gc_timer = Some(timer));
    }

    pub(crate) fn fetch(&self, mode: QueryFetchMode) {
        let now = now_ms();
        if mode == QueryFetchMode::Normal
            && self
                .resource
                .with_untracked(|r| r.should_short_circuit_cache(now))
        {
            return;
        }
        let mut started = None;
        self.resource.update(|r| {
            let active_before = r.active_request_id();
            started = match r.begin_request_with_id(None, now, mode) {
                QueryBeginResult::StaleCacheHit {
                    request_id,
                    replaced_request_id: None,
                    ..
                } if Some(request_id) == active_before => None,
                QueryBeginResult::Started { request_id, .. }
                | QueryBeginResult::StaleCacheHit { request_id, .. } => Some(request_id),
                QueryBeginResult::CacheHit | QueryBeginResult::IgnoredWhileLoading { .. } => None,
            };
        });
        if let Some(request_id) = started {
            self.run(request_id);
        }
    }

    /// A forced load that replaces one in flight, whatever the request policy.
    fn reload(&self) {
        let policy = self.resource.with_untracked(|r| r.request_policy());
        self.resource
            .update(|r| r.set_request_policy(RequestPolicy::LatestWins));
        self.fetch(QueryFetchMode::Force);
        self.resource.update(|r| r.set_request_policy(policy));
    }

    fn run(&self, request_id: RequestId) {
        let entry = *self;
        let task = self
            .owner
            .run(|| spawn(async move { entry.attempts(request_id).await }));
        let previous = self.shared.with(|s| s.running.replace(task));
        if let Some(previous) = previous {
            previous.cancel();
        }
    }

    async fn attempts(self, request_id: RequestId) {
        let Some(fetcher) = self.shared.with(|s| s.fetcher.clone()) else {
            return;
        };
        let retry = self.shared.with(|s| s.retry.clone());
        let mut attempt = 0;
        loop {
            let (key, signal) = self
                .resource
                .with_untracked(|r| (r.key().clone(), r.signal().cloned().unwrap_or_default()));
            let result = fetcher(QueryContext { key, signal }).await;
            if !self
                .resource
                .with_untracked(|r| r.is_current_request(request_id))
            {
                self.resource.update(QueryResource::mark_ignored_result);
                return;
            }
            match result {
                Ok(data) => {
                    let now = now_ms();
                    self.resource.update(|r| {
                        r.reset_retry_count();
                        r.complete_current_success(request_id, data, now);
                    });
                    return;
                }
                Err(_) if retry.should_retry(attempt) => {
                    let delay = retry.delay_for_attempt(attempt);
                    attempt += 1;
                    self.resource.update(QueryResource::increment_retry);
                    timer(Duration::from_millis(delay)).await;
                }
                Err(error) => {
                    let now = now_ms();
                    self.resource.update(|r| {
                        r.complete_current_failure(request_id, error, now);
                    });
                    return;
                }
            }
        }
    }
}

/// A query a component observes. `Copy`; reads are tracked, so use them in bindings.
pub struct Query<T: 'static, E: 'static = QueryError> {
    current: Signal<Entry<T, E>>,
}

impl<T, E> Clone for Query<T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, E> Copy for Query<T, E> {}

impl<T: 'static, E: 'static> Query<T, E> {
    fn with_resource<R>(&self, f: impl FnOnce(&QueryResource<T, E>) -> R) -> R {
        self.current.get().resource.with(f)
    }

    /// Reads the data without cloning it. `f` gets `None` until the first success.
    pub fn with<R>(&self, f: impl FnOnce(Option<&T>) -> R) -> R {
        self.with_resource(|r| f(r.data()))
    }

    pub fn data(&self) -> Option<T>
    where
        T: Clone,
    {
        self.with_resource(|r| r.data().cloned())
    }

    pub fn error(&self) -> Option<E>
    where
        E: Clone,
    {
        self.with_resource(|r| r.error().cloned())
    }

    pub fn status(&self) -> QueryStatus {
        self.with_resource(QueryResource::status)
    }

    /// True until the first load ends: there is nothing to show yet.
    pub fn is_pending(&self) -> bool {
        self.with_resource(QueryResource::is_pending)
    }

    /// True during any load, the background refetch of existing data included.
    pub fn is_fetching(&self) -> bool {
        self.with_resource(QueryResource::is_loading)
    }

    pub fn is_success(&self) -> bool {
        self.status() == QueryStatus::Success
    }

    pub fn is_failure(&self) -> bool {
        self.status() == QueryStatus::Failure
    }

    pub fn retry_count(&self) -> u32 {
        self.with_resource(QueryResource::retry_count)
    }

    pub fn last_updated_at_ms(&self) -> Option<u64> {
        self.with_resource(QueryResource::last_updated_at_ms)
    }

    pub fn key(&self) -> QueryKey {
        self.with_resource(|r| r.key().clone())
    }

    /// Loads again now, ignoring freshness. One load at a time unless the
    /// options chose `RequestPolicy::LatestWins`.
    pub fn refetch(&self) {
        self.current.get_untracked().fetch(QueryFetchMode::Force);
    }

    /// Replaces the data, keeping the old value for [`Self::rollback`]. For
    /// optimistic updates.
    pub fn set_data(&self, data: T) {
        self.current
            .get_untracked()
            .resource
            .update(|r| r.set_data(data));
    }

    /// Puts back the value [`Self::set_data`] replaced. False when there is none.
    pub fn rollback(&self) -> bool {
        let mut rolled = false;
        self.current
            .get_untracked()
            .resource
            .update(|r| rolled = r.rollback_to_previous());
        rolled
    }

    /// A memo of part of the data. Bindings on it run only when that part changes.
    pub fn select<U: PartialEq + 'static>(&self, f: impl Fn(&T) -> U + 'static) -> Memo<Option<U>> {
        let query = *self;
        computed(move || query.with(|data| data.map(&f)))
    }
}

/// Observes `key` with default options. The fetcher runs when nothing fresh is cached.
pub fn use_query<T, E, F, Fut>(key: impl Into<QueryKey>, fetcher: F) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    use_query_with(key, QueryOptions::default(), fetcher)
}

/// [`use_query`] with options.
pub fn use_query_with<T, E, F, Fut>(
    key: impl Into<QueryKey>,
    options: QueryOptions,
    fetcher: F,
) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    let key = key.into();
    use_query_by(move || key.clone(), options, fetcher)
}

/// Observes a key that can change: `key` is tracked, and the fetcher reads the
/// current one from its [`QueryContext`]. Moving back to a key serves its cache.
pub fn use_query_by<T, E, K, F, Fut>(
    key: impl Fn() -> K + 'static,
    options: QueryOptions,
    fetcher: F,
) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    K: Into<QueryKey>,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    let client = query_client();
    let fetcher: Fetcher<T, E> = Rc::new(move |context| Box::pin(fetcher(context)));
    let initial = untracked(|| key().into());
    let first = Entry::<T, E>::get_or_create(&client, &initial, &options);
    first.set_fetcher(fetcher.clone());
    first.observe();
    let current = signal_with(first, SignalEquals::Never);
    // A component's signals are gone by the time its cleanups run.
    let observed = Rc::new(Cell::new(first));
    let released = observed.clone();
    let enabled = options.enabled.clone();
    let setup = options.clone();

    effect(move || {
        let next_key: QueryKey = key().into();
        let on = enabled.as_ref().is_none_or(|when| when());
        untracked(|| {
            let shown = current.get_untracked();
            let entry = if AnyEntry::key(&shown) == next_key {
                shown
            } else {
                let next = Entry::<T, E>::get_or_create(&client, &next_key, &setup);
                next.set_fetcher(fetcher.clone());
                next.observe();
                current.set(next);
                observed.set(next);
                shown.release();
                next
            };
            if on {
                entry.fetch(QueryFetchMode::Normal);
            }
        });
    });

    cleanup(move || released.get().release());

    if let Some(every) = options.refetch_interval {
        drop(set_interval(every, move || {
            if lifecycle() != Lifecycle::Background {
                current.get_untracked().fetch(QueryFetchMode::Force);
            }
        }));
    }

    if options.refetch_on_foreground != RefetchTrigger::Never {
        let trigger = options.refetch_on_foreground;
        let was = stored(untracked(lifecycle));
        effect(move || {
            let now = lifecycle();
            let before = was.replace(now);
            if now == Lifecycle::Active && before == Lifecycle::Background {
                let mode = match trigger {
                    RefetchTrigger::Always => QueryFetchMode::Force,
                    _ => QueryFetchMode::Normal,
                };
                untracked(|| current.get_untracked().fetch(mode));
            }
        });
    }

    Query { current }
}
