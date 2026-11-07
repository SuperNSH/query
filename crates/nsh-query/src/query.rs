use std::any::{Any, TypeId};
use std::cell::Cell;
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::Poll;
use std::time::Duration;

use gpui_query::{
    Fetched, NetworkMode, QueryBeginResult, QueryError, QueryFetchMode, QueryKey, QueryResource,
    RefetchTrigger, RequestId, RequestPolicy, RetryPolicy,
};
use nsh_core::JsonValue;
use nsh_core::element::{create_root, set_interval, set_timeout};
use nsh_core::prelude::*;

use crate::client::{AnyEntry, QueryClient, QueryState, query_client};
use crate::diagnostics::QueryDiagnostic;
use crate::managers::{focus_manager, online_manager, wait_online};
use crate::now_ms;
use crate::options::{
    DataEqual, Persister, QueryConfig, QueryContext, QueryOptions, RetryDelay, RetryIf,
};
use crate::status::{FetchStatus, Outcome, Status};
use crate::tracked::Tracked;

/// The fetcher of one entry. Its `Fetched` may override the cache policy on success.
pub(crate) type Fetcher<T, E> =
    Rc<dyn Fn(QueryContext) -> Pin<Box<dyn Future<Output = Result<Fetched<T>, E>>>>>;

/// Wraps a fetcher that returns plain data, so the entry takes no policy from it.
pub(crate) fn plain_fetcher<T, E, F, Fut>(fetcher: F) -> Fetcher<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    Rc::new(move |context| {
        let future = fetcher(context);
        Box::pin(async move { future.await.map(Fetched::new) })
    })
}

/// Wraps a fetcher that returns [`Fetched`], whose cache policy replaces the entry's on success.
pub(crate) fn policy_fetcher<T, E, F, Fut>(fetcher: F) -> Fetcher<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<Fetched<T>, E>> + 'static,
{
    Rc::new(move |context| Box::pin(fetcher(context)))
}

static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// The bookkeeping TanStack keeps beside the data: counts, timestamps and
/// the outcome that sets the status.
pub(crate) struct Meta<E> {
    pub(crate) outcome: Outcome,
    pub(crate) fetching: bool,
    pub(crate) paused: bool,
    pub(crate) invalidated: bool,
    pub(crate) data_updated_at: u64,
    pub(crate) error_updated_at: u64,
    pub(crate) data_update_count: u32,
    pub(crate) error_update_count: u32,
    pub(crate) failure_count: u32,
    /// The error of the attempt being retried. Once the load gives up, it moves
    /// into the resource, which then holds the failure reason.
    pub(crate) failure_reason: Option<E>,
    /// Bumped only when the data changes. The data has no `PartialEq`, so its readers follow this.
    pub(crate) data_version: u64,
}

impl<E> Default for Meta<E> {
    fn default() -> Self {
        Self {
            outcome: Outcome::Pending,
            fetching: false,
            paused: false,
            invalidated: false,
            data_updated_at: 0,
            error_updated_at: 0,
            data_update_count: 0,
            error_update_count: 0,
            failure_count: 0,
            failure_reason: None,
            data_version: 0,
        }
    }
}

struct Shared<T: 'static, E: 'static> {
    fetcher: Option<Fetcher<T, E>>,
    retry: RetryPolicy,
    retry_if: Option<RetryIf<E>>,
    retry_delay: Option<RetryDelay<E>>,
    network: NetworkMode,
    reconnect: RefetchTrigger,
    persist: Option<Persister<T>>,
    equal: Option<DataEqual<T>>,
    gc: Duration,
    meta: Option<JsonValue>,
    /// Observers that follow the entry. Only these keep it out of gc.
    observers: usize,
    /// Handles that read the entry without following it, as `subscribed(false)` does.
    /// They keep the entry's signals alive, but never stop its gc.
    holders: usize,
    gc_timer: Option<Spawned<()>>,
    running: Option<Spawned<()>>,
    waiters: Vec<std::task::Waker>,
}

impl<T: 'static, E: 'static> Shared<T, E> {
    fn from_config(config: &QueryConfig<T, E>) -> Self {
        Self {
            fetcher: None,
            retry: config.options.retry.clone(),
            retry_if: config.retry_if.clone(),
            retry_delay: config.retry_delay.clone(),
            network: config.options.network_mode,
            reconnect: config.options.refetch_on_reconnect,
            persist: config.persist,
            equal: config.equal.clone(),
            gc: config.options.gc,
            meta: config.options.meta.clone(),
            observers: 0,
            holders: 0,
            gc_timer: None,
            running: None,
            waiters: Vec::new(),
        }
    }
}

/// One cached query: its state machine as a signal, under an owner of its own
/// so the entry outlives the components observing it.
pub(crate) struct Entry<T: 'static, E: 'static> {
    uid: u64,
    resource: Tracked<QueryResource<T, E>>,
    meta: Tracked<Meta<E>>,
    /// One memo per value a public read shows. A binding subscribes only to the
    /// memos it reads, so it wakes when one of those values changes.
    fields: Fields,
    shared: Stored<Shared<T, E>>,
    owner: Owner,
}

/// The memos over an entry's cell. Each is `computed`, so its readers run again
/// only when its own value changes, not when another field of the entry does.
#[derive(Clone, Copy)]
pub(crate) struct Fields {
    /// Bumped only when the data itself is written, since data has no `PartialEq`.
    pub(crate) data_version: Memo<u64>,
    pub(crate) has_data: Memo<bool>,
    pub(crate) outcome: Memo<Outcome>,
    pub(crate) fetching: Memo<bool>,
    pub(crate) paused: Memo<bool>,
    pub(crate) failures: Memo<u32>,
    /// Whether an error is held, and how many loads ended in one.
    pub(crate) error_key: Memo<(bool, u32)>,
    pub(crate) data_at: Memo<u64>,
    pub(crate) error_at: Memo<u64>,
    pub(crate) data_count: Memo<u32>,
    pub(crate) error_count: Memo<u32>,
    pub(crate) invalidated: Memo<bool>,
    pub(crate) core_status: Memo<gpui_query::QueryStatus>,
    pub(crate) core_pending: Memo<bool>,
    pub(crate) retries: Memo<u32>,
    pub(crate) last_updated: Memo<Option<u64>>,
}

impl Fields {
    fn new<T: 'static, E: 'static>(
        resource: Tracked<QueryResource<T, E>>,
        meta: Tracked<Meta<E>>,
    ) -> Self {
        Self {
            data_version: computed(move || meta.with(|m| m.data_version)),
            has_data: computed(move || resource.with(|r| r.has_data())),
            outcome: computed(move || meta.with(|m| m.outcome)),
            fetching: computed(move || meta.with(|m| m.fetching)),
            paused: computed(move || meta.with(|m| m.paused)),
            failures: computed(move || meta.with(|m| m.failure_count)),
            error_key: computed(move || {
                (
                    resource.with(|r| r.error().is_some()),
                    meta.with(|m| m.error_update_count),
                )
            }),
            data_at: computed(move || meta.with(|m| m.data_updated_at)),
            error_at: computed(move || meta.with(|m| m.error_updated_at)),
            data_count: computed(move || meta.with(|m| m.data_update_count)),
            error_count: computed(move || meta.with(|m| m.error_update_count)),
            invalidated: computed(move || meta.with(|m| m.invalidated)),
            core_status: computed(move || resource.with(|r| r.status())),
            core_pending: computed(move || resource.with(|r| r.is_pending())),
            retries: computed(move || resource.with(|r| r.retry_count())),
            last_updated: computed(move || resource.with(|r| r.last_updated_at_ms())),
        }
    }
}

impl<T, E> Clone for Entry<T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, E> Copy for Entry<T, E> {}

impl<T: 'static, E: 'static> AnyEntry for Entry<T, E> {
    fn uid(&self) -> u64 {
        self.uid
    }

    fn state(&self) -> QueryState {
        let now = now_ms();
        let fresh = self.resource.with_untracked(|r| r.is_cache_fresh(now));
        let has_data = self.resource.with_untracked(|r| r.has_data());
        let key = self.resource.with_untracked(|r| r.key().clone());
        let m = self.meta.with_untracked(|m| {
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
        let (outcome, fetching, paused, invalidated, data_at, error_at, data_n, error_n, failures) =
            m;
        QueryState {
            key,
            status: outcome.status(),
            fetch_status: fetch_status_of(fetching, paused),
            data_updated_at: data_at,
            error_updated_at: error_at,
            data_update_count: data_n,
            error_update_count: error_n,
            failure_count: failures,
            is_invalidated: invalidated,
            is_stale: !fresh,
            observers: self.shared.with(|s| s.observers),
            has_data,
        }
    }

    fn invalidate(&self) {
        self.resource.update(QueryResource::invalidate);
        self.meta.update(|m| m.invalidated = true);
        query_client().touch();
        if self.shared.with(|s| s.observers) > 0 {
            self.reload();
        }
    }

    fn cancel(&self) {
        (*self).cancel_inner();
    }

    fn reset(&self) {
        self.cancel_inner();
        self.resource.update(QueryResource::reset);
        self.meta.update(|m| {
            // The version only grows, so a reader sees the reset as a change.
            let version = m.data_version;
            *m = Meta::default();
            m.data_version = version + 1;
        });
        query_client().touch();
        if self.shared.with(|s| s.observers) > 0 {
            self.fetch(QueryFetchMode::Force);
        }
    }

    fn refetch(&self) {
        self.fetch(QueryFetchMode::Force);
    }

    fn dispose(&self) {
        self.owner.dispose();
    }

    fn is_held(&self) -> bool {
        self.shared.with(|s| s.observers > 0 || s.holders > 0)
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
        let persist = self.shared.with(|s| s.persist)?;
        let value = self
            .resource
            .with_untracked(|r| r.data().and_then(|d| (persist.ser)(d)))?;
        Some((value, self.meta.with_untracked(|m| m.data_updated_at)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn fetch_status_of(fetching: bool, paused: bool) -> FetchStatus {
    match (fetching, paused) {
        (true, true) => FetchStatus::Paused,
        (true, false) => FetchStatus::Fetching,
        _ => FetchStatus::Idle,
    }
}

impl<T: 'static, E: 'static> Entry<T, E> {
    fn id(key: &QueryKey) -> (TypeId, QueryKey) {
        (TypeId::of::<(T, E)>(), key.clone())
    }

    pub(crate) fn find(client: &QueryClient, key: &QueryKey) -> Option<Self> {
        client.lookup::<Self>(&Self::id(key))
    }

    /// The entry for `key`, created from `config` when none is cached. A cached
    /// entry takes the new config for its next load.
    pub(crate) fn get_or_create(
        client: &QueryClient,
        key: &QueryKey,
        config: &QueryConfig<T, E>,
    ) -> Self {
        if let Some(entry) = Self::find(client, key) {
            entry.configure(config);
            return entry;
        }
        let uid = NEXT_UID.fetch_add(1, Ordering::Relaxed);
        let (owner, entry) = create_root(|| {
            let mut resource = QueryResource::<T, E>::new(
                key.clone(),
                config.options.cache,
                config.options.request,
            );
            resource.set_retry_policy(config.options.retry.clone());
            let resource = Tracked::new(resource);
            let meta = Tracked::new(Meta::default());
            Entry {
                uid,
                resource,
                meta,
                fields: Fields::new(resource, meta),
                shared: stored(Shared::from_config(config)),
                owner: Owner::current(),
            }
        });
        let entry = Entry { owner, ..entry };
        client.insert(Self::id(key), Rc::new(entry));
        // Nothing observes a new entry yet, so its gc starts now, as in TanStack.
        entry.settle();
        entry.owner.run(|| entry.install_reconnect());

        let hydrated = client.take_hydrated(key);
        let restored = match (hydrated, config.persist) {
            (Some((value, at)), Some(persist)) => (persist.de)(value).map(|data| (data, at)),
            _ => None,
        };
        if let Some((data, at)) = restored {
            entry.write_at(data, at, false);
        } else if let Some(initial) = &config.initial_data {
            let at = config
                .options
                .initial_data_updated_at
                .unwrap_or_else(now_ms);
            entry.write_at(initial(), at, false);
        }
        entry
    }

    fn configure(&self, config: &QueryConfig<T, E>) {
        self.shared.with(|s| {
            s.retry = config.options.retry.clone();
            s.retry_if = config.retry_if.clone();
            s.retry_delay = config.retry_delay.clone();
            s.network = config.options.network_mode;
            s.reconnect = config.options.refetch_on_reconnect;
            s.persist = config.persist;
            s.equal = config.equal.clone();
            s.gc = config.options.gc;
            s.meta = config.options.meta.clone();
        });
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

    /// Calls `f` with the data by reference. Tracked on the data version, so the
    /// caller re-runs when the data is written.
    pub(crate) fn with_data<R>(&self, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.fields.data_version.get();
        self.resource.with_untracked(|r| r.data().map(f))
    }

    /// A write that counts as a fetch: TanStack's `setQueryData`.
    pub(crate) fn write(&self, data: T, at: u64, counted: bool) {
        self.write_at(data, at, counted);
    }

    fn write_at(&self, data: T, at: u64, counted: bool) {
        self.resource.update(|r| {
            let policy = r.request_policy();
            r.set_request_policy(RequestPolicy::LatestWins);
            if let QueryBeginResult::Started { request_id, .. } =
                r.begin_request_with_id(None, at, QueryFetchMode::Force)
            {
                r.reset_retry_count();
                r.complete_current_success(request_id, data, at);
            }
            r.set_request_policy(policy);
        });
        self.meta.update(|m| {
            m.fetching = false;
            m.paused = false;
            m.outcome = Outcome::Success;
            m.data_updated_at = at;
            m.failure_count = 0;
            m.failure_reason = None;
            m.invalidated = false;
            m.data_version += 1;
            if counted {
                m.data_update_count += 1;
            }
        });
        self.wake_waiters();
        query_client().touch();
    }

    /// Rewrites the data from `updater`. False when the updater returns `None`.
    pub(crate) fn update_data(&self, updater: impl FnOnce(Option<&T>) -> Option<T>) -> bool {
        let next = self.resource.with_untracked(|r| updater(r.data()));
        match next {
            Some(data) => {
                self.write(data, now_ms(), true);
                true
            }
            None => false,
        }
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

    /// An observer lets go. The gc starts once no observer is left.
    fn release(&self) {
        self.shared
            .with(|s| s.observers = s.observers.saturating_sub(1));
        query_client().touch();
        self.settle();
    }

    /// A handle reads the entry without following it, as `subscribed(false)` does. It
    /// restarts the gc timer, so the handle's gc time applies to an entry made earlier.
    fn hold(&self) {
        self.shared.with(|s| s.holders += 1);
        self.settle();
    }

    fn unhold(&self) {
        self.shared
            .with(|s| s.holders = s.holders.saturating_sub(1));
        self.settle();
    }

    /// Starts the gc timer while no observer follows the entry. Once the cache has dropped the
    /// entry and no handle reads it, its owner ends.
    fn settle(&self) {
        let (observers, held, gc) = self.shared.with(|s| (s.observers, s.holders, s.gc));
        if observers > 0 {
            return;
        }
        if !query_client().is_registered(self.uid) {
            if held == 0 {
                self.owner.dispose();
            }
            return;
        }
        let entry = *self;
        let id = Self::id(&entry.key());
        let uid = self.uid;
        let timer = self.owner.run(|| {
            set_timeout(gc, move || {
                query_client().evict_uid(&id, uid);
            })
        });
        let previous = self.shared.with(|s| s.gc_timer.replace(timer));
        if let Some(previous) = previous {
            previous.cancel();
        }
    }

    fn key(&self) -> QueryKey {
        self.resource.with_untracked(|r| r.key().clone())
    }

    fn install_reconnect(self) {
        let was = stored(online_manager().is_online());
        effect(move || {
            let now = online_manager().is_online();
            let before = was.replace(now);
            if now && !before {
                untracked(|| match self.shared.with(|s| s.reconnect) {
                    RefetchTrigger::Always => self.fetch(QueryFetchMode::Force),
                    RefetchTrigger::IfStale => self.fetch(QueryFetchMode::Normal),
                    RefetchTrigger::Never => {}
                });
            }
        });
    }

    /// Starts a load unless the cache serves one. A load that runs while the
    /// app is offline is paused, not dropped.
    pub(crate) fn fetch(&self, mode: QueryFetchMode) {
        if !self.shared.with(|s| s.fetcher.is_some()) {
            return;
        }
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
            self.meta.update(|m| {
                m.fetching = true;
                m.failure_count = 0;
                m.failure_reason = None;
            });
            query_client().touch();
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

    /// Resolves once no load of this entry is running.
    pub(crate) async fn wait_idle(self) {
        poll_fn(move |cx| {
            if !self.meta.with_untracked(|m| m.fetching) {
                return Poll::Ready(());
            }
            self.shared.with(|s| s.waiters.push(cx.waker().clone()));
            Poll::Pending
        })
        .await
    }

    fn cancel_inner(&self) {
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

    /// Ends a load that has no fetcher to run.
    fn abandon(&self, request_id: RequestId) {
        self.resource.update(|r| {
            let _ = r.accept_current_request(request_id);
        });
        self.meta.update(|m| m.fetching = false);
        query_client().touch();
        self.wake_waiters();
    }

    /// Stores a successful load. A policy the fetcher returned replaces the entry's.
    fn commit(&self, request_id: RequestId, fetched: Fetched<T>, now: u64) {
        let Fetched {
            data, cache_policy, ..
        } = fetched;
        let same = self.shared.with(|s| s.equal.clone()).is_some_and(|equal| {
            self.resource
                .with_untracked(|r| r.data().is_some_and(|old| equal(old, &data)))
        });
        // The data is written before its version moves: a binding that runs on
        // the version then reads the new value.
        batch(|| {
            self.resource.update(|r| {
                if let Some(policy) = cache_policy {
                    r.set_cache_policy(policy);
                }
                r.reset_retry_count();
                r.complete_current_success(request_id, data, now);
            });
            self.meta.update(|m| {
                if !same {
                    m.data_version += 1;
                }
                m.fetching = false;
                m.outcome = Outcome::Success;
                m.data_updated_at = now;
                m.data_update_count += 1;
                m.failure_count = 0;
                m.failure_reason = None;
                m.invalidated = false;
            });
        });
        query_client().touch();
        self.wake_waiters();
    }

    async fn attempts(self, request_id: RequestId) {
        let Some(fetcher) = self.shared.with(|s| s.fetcher.clone()) else {
            self.abandon(request_id);
            return;
        };
        let (retry, retry_if, retry_delay, network) = self.shared.with(|s| {
            (
                s.retry.clone(),
                s.retry_if.clone(),
                s.retry_delay.clone(),
                s.network,
            )
        });
        loop {
            let failures = self.meta.with_untracked(|m| m.failure_count);
            let pause = !online_manager().is_online()
                && match network {
                    NetworkMode::Online => true,
                    NetworkMode::OfflineFirst => failures > 0,
                    NetworkMode::Always => false,
                };
            if pause {
                self.meta.update(|m| m.paused = true);
                query_client().touch();
                wait_online(false).await;
                self.meta.update(|m| m.paused = false);
            }
            if !self.is_current(request_id) {
                self.resource.update(QueryResource::mark_ignored_result);
                return;
            }
            let (key, signal) = self
                .resource
                .with_untracked(|r| (r.key().clone(), r.signal().cloned().unwrap_or_default()));
            let meta = self.shared.with(|s| s.meta.clone());
            let result = fetcher(QueryContext {
                key,
                signal,
                meta,
                client: query_client(),
                direction: None,
            })
            .await;
            if !self.is_current(request_id) {
                self.resource.update(QueryResource::mark_ignored_result);
                return;
            }
            match result {
                Ok(fetched) => {
                    self.commit(request_id, fetched, now_ms());
                    query_client().notify_query(&self.state(), true);
                    return;
                }
                Err(error) => {
                    self.meta.update(|m| {
                        m.failure_count += 1;
                        m.failure_reason = Some(error);
                    });
                    let count = self.meta.with_untracked(|m| m.failure_count);
                    let retry_now = self.meta.with_untracked(|m| {
                        let reason = m.failure_reason.as_ref();
                        match (&retry_if, reason) {
                            (Some(decide), Some(reason)) => decide(count - 1, reason),
                            _ => retry.should_retry(count - 1),
                        }
                    });
                    if retry_now {
                        let delay = self.meta.with_untracked(|m| {
                            match (&retry_delay, m.failure_reason.as_ref()) {
                                (Some(delay), Some(reason)) => delay(count - 1, reason),
                                _ => Duration::from_millis(retry.delay_for_attempt(count - 1)),
                            }
                        });
                        self.resource.update(QueryResource::increment_retry);
                        query_client().touch();
                        timer(delay).await;
                        continue;
                    }
                    let now = now_ms();
                    let mut taken = None;
                    self.meta.update(|m| {
                        taken = m.failure_reason.take();
                        m.fetching = false;
                        m.outcome = Outcome::Error;
                        m.error_updated_at = now;
                        m.error_update_count += 1;
                    });
                    if let Some(error) = taken {
                        self.resource.update(|r| {
                            r.complete_current_failure(request_id, error, now);
                        });
                    }
                    query_client().touch();
                    query_client().notify_query(&self.state(), false);
                    self.wake_waiters();
                    return;
                }
            }
        }
    }
}

/// Why [`QueryClient::fetch_query`] did not return data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchQueryError<E> {
    /// The last attempt failed with this error.
    Failed(E),
    /// The load was cancelled, or superseded, before it settled.
    Cancelled,
}

impl QueryClient {
    /// Returns the cached data when it is fresh, else loads it and returns the result.
    /// Unlike TanStack, a cancelled load returns [`FetchQueryError::Cancelled`].
    pub async fn fetch_query<T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: impl Into<QueryConfig<T, E>>,
        fetcher: F,
    ) -> Result<T, FetchQueryError<E>>
    where
        T: Clone + 'static,
        E: Clone + 'static,
        F: Fn(QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        self.fetch_query_with(key, options, plain_fetcher(fetcher))
            .await
    }

    /// [`Self::fetch_query`] whose fetcher returns [`Fetched`]. The `Fetched`
    /// cache policy, when set, replaces the entry's on success, as in
    /// [`use_query_with_policy`](crate::use_query_with_policy).
    pub async fn fetch_query_with_policy<T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: impl Into<QueryConfig<T, E>>,
        fetcher: F,
    ) -> Result<T, FetchQueryError<E>>
    where
        T: Clone + 'static,
        E: Clone + 'static,
        F: Fn(QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<Fetched<T>, E>> + 'static,
    {
        self.fetch_query_with(key, options, policy_fetcher(fetcher))
            .await
    }

    async fn fetch_query_with<T, E>(
        &self,
        key: impl Into<QueryKey>,
        options: impl Into<QueryConfig<T, E>>,
        fetcher: Fetcher<T, E>,
    ) -> Result<T, FetchQueryError<E>>
    where
        T: Clone + 'static,
        E: Clone + 'static,
    {
        let key = key.into();
        let config = self.resolve_query_config::<T, E>(&key, &options.into());
        let entry = Entry::<T, E>::get_or_create(self, &key, &config);
        entry.set_fetcher(fetcher);
        entry.fetch(QueryFetchMode::Normal);
        entry.wait_idle().await;
        settled_result(entry)
    }

    /// TanStack's `client.query(options)`: the cached data while it is fresh by its
    /// stale time, else a fetch. As in TanStack, the fetch is not retried unless
    /// `retry` is set, so the default policy becomes no retries. `enabled` is not read
    /// here, as TanStack's `query` does not read it either; only observers read it.
    pub async fn query<T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        config: impl Into<QueryConfig<T, E>>,
        fetcher: F,
    ) -> Result<T, FetchQueryError<E>>
    where
        T: Clone + 'static,
        E: Clone + 'static,
        F: Fn(QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        let mut config: QueryConfig<T, E> = config.into();
        if config.options.retry == RetryPolicy::default() {
            config.options.retry = RetryPolicy::no_retries();
        }
        self.fetch_query(key, config, fetcher).await
    }

    /// [`Self::query`] with TanStack's `select`, applied to the data before it returns.
    pub async fn query_selected<T, E, U, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        config: impl Into<QueryConfig<T, E>>,
        fetcher: F,
        select: impl Fn(&T) -> U,
    ) -> Result<U, FetchQueryError<E>>
    where
        T: Clone + 'static,
        E: Clone + 'static,
        F: Fn(QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        let data = self.query(key, config, fetcher).await?;
        Ok(select(&data))
    }

    /// Returns the cached data as is, or loads it when nothing is cached.
    pub async fn ensure_query_data<T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: impl Into<QueryConfig<T, E>>,
        fetcher: F,
    ) -> Result<T, FetchQueryError<E>>
    where
        T: Clone + 'static,
        E: Clone + 'static,
        F: Fn(QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        let key = key.into();
        if let Some(data) = self.data::<T, E>(key.clone()) {
            return Ok(data);
        }
        self.fetch_query(key, options, fetcher).await
    }
}

fn settled_result<T: Clone + 'static, E: Clone + 'static>(
    entry: Entry<T, E>,
) -> Result<T, FetchQueryError<E>> {
    let outcome = entry.meta.with_untracked(|m| m.outcome);
    match outcome {
        Outcome::Success => entry.data_untracked().ok_or(FetchQueryError::Cancelled),
        Outcome::Error => entry
            .resource
            .with_untracked(|r| r.error().cloned())
            .map(FetchQueryError::Failed)
            .map_or(Err(FetchQueryError::Cancelled), Err),
        Outcome::Pending => match entry.data_untracked() {
            Some(data) => Ok(data),
            None => Err(FetchQueryError::Cancelled),
        },
    }
}

/// A query a component observes. `Copy`; reads are tracked, so use them in bindings.
pub struct Query<T: 'static, E: 'static = QueryError> {
    current: Signal<Entry<T, E>>,
    enabled: Signal<bool>,
    placeholder: Tracked<Option<T>>,
    previous: Tracked<Option<Entry<T, E>>>,
    keep_previous: bool,
    throw: Option<fn(&E) -> String>,
}

impl<T, E> Clone for Query<T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, E> Copy for Query<T, E> {}

impl<T: 'static, E: 'static> Query<T, E> {
    fn entry(&self) -> Entry<T, E> {
        self.current.get()
    }

    /// The data a binding sees: the cached value, else placeholder data, else
    /// the previous key's data when `keep_previous_data` is set. Subscribes to
    /// the data's own version and to the memos that decide what is shown.
    pub fn with<R>(&self, f: impl FnOnce(Option<&T>) -> R) -> R {
        let entry = self.entry();
        if let Some(message) = self.throw {
            // The error lands in a second write after the outcome, so follow its memo too.
            entry.fields.error_key.get();
            if entry.fields.outcome.get() == Outcome::Error
                && !entry.fields.fetching.get()
                && let Some(text) = entry.resource.with_untracked(|r| r.error().map(message))
            {
                panic!("{text}");
            }
        }
        entry.fields.data_version.get();
        if entry.fields.has_data.get() {
            return entry.resource.with_untracked(|r| f(r.data()));
        }
        if entry.fields.outcome.get() != Outcome::Pending {
            return f(None);
        }
        // Placeholder and previous are read untracked: each changes only with the key, and a
        // key change always sets `current`, which every reader follows.
        if self.placeholder.with_untracked(|p| p.is_some()) {
            return self.placeholder.with_untracked(|p| f(p.as_ref()));
        }
        if self.keep_previous
            && let Some(prev) = self.previous.with_untracked(|p| *p)
            && prev.owner.is_live()
        {
            prev.fields.data_version.get();
            if prev.resource.with_untracked(|r| r.data().is_some()) {
                return prev.resource.with_untracked(|r| f(r.data()));
            }
        }
        f(None)
    }

    pub fn data(&self) -> Option<T>
    where
        T: Clone,
    {
        self.with(|d| d.cloned())
    }

    pub fn error(&self) -> Option<E>
    where
        E: Clone,
    {
        let entry = self.entry();
        entry.fields.error_key.get();
        entry.resource.with_untracked(|r| r.error().cloned())
    }

    /// The core state machine's status. Use [`Self::status_v5`] for TanStack's.
    pub fn status(&self) -> gpui_query::QueryStatus {
        self.entry().fields.core_status.get()
    }

    /// TanStack's `status`: `Pending`, `Error` or `Success`.
    pub fn status_v5(&self) -> Status {
        if self.placeholder_active() {
            return Status::Success;
        }
        self.entry().fields.outcome.get().status()
    }

    /// TanStack's `fetchStatus`.
    pub fn fetch_status(&self) -> FetchStatus {
        let entry = self.entry();
        fetch_status_of(entry.fields.fetching.get(), entry.fields.paused.get())
    }

    /// True until the first load ends: there is nothing to show yet.
    pub fn is_pending(&self) -> bool {
        self.entry().fields.core_pending.get()
    }

    /// True while a load runs, including a background refetch. A paused load is not fetching.
    pub fn is_fetching(&self) -> bool {
        self.fetch_status() == FetchStatus::Fetching
    }

    pub fn is_success(&self) -> bool {
        self.status_v5() == Status::Success
    }

    pub fn is_failure(&self) -> bool {
        self.status_v5() == Status::Error
    }

    pub fn retry_count(&self) -> u32 {
        self.entry().fields.retries.get()
    }

    pub fn last_updated_at_ms(&self) -> Option<u64> {
        self.entry().fields.last_updated.get()
    }

    pub fn key(&self) -> QueryKey {
        self.entry().resource.with_untracked(|r| r.key().clone())
    }

    /// When the data last loaded, in ms since the Unix epoch. 0 before any data.
    pub fn data_updated_at(&self) -> u64 {
        self.entry().fields.data_at.get()
    }

    /// When the last error happened. 0 before any error.
    pub fn error_updated_at(&self) -> u64 {
        self.entry().fields.error_at.get()
    }

    /// Failed attempts in the current load. Resets when a load starts or succeeds.
    pub fn failure_count(&self) -> u32 {
        self.entry().fields.failures.get()
    }

    /// The error of the latest failed attempt, which a retry may still turn into success.
    pub fn failure_reason(&self) -> Option<E>
    where
        E: Clone,
    {
        let entry = self.entry();
        entry.fields.failures.get();
        entry.fields.error_key.get();
        if let Some(reason) = entry.meta.with_untracked(|m| m.failure_reason.clone()) {
            return Some(reason);
        }
        if entry.meta.with_untracked(|m| m.outcome) == Outcome::Error {
            return entry.resource.with_untracked(|r| r.error().cloned());
        }
        None
    }

    /// How many loads ended in an error.
    pub fn error_update_count(&self) -> u32 {
        self.entry().fields.error_count.get()
    }

    pub fn is_error(&self) -> bool {
        self.status_v5() == Status::Error
    }

    /// True once the query has settled, with data or an error.
    pub fn is_fetched(&self) -> bool {
        let entry = self.entry();
        entry.fields.data_count.get() + entry.fields.error_count.get() > 0
    }

    /// Same as [`Self::is_fetched`]: the entry's counts start at zero when it is created.
    pub fn is_fetched_after_mount(&self) -> bool {
        self.is_fetched()
    }

    /// `is_fetching() && is_pending()`: the first load is running.
    pub fn is_loading(&self) -> bool {
        self.is_fetching() && self.status_v5() == Status::Pending
    }

    /// Same as [`Self::is_loading`], under TanStack's older name.
    pub fn is_initial_loading(&self) -> bool {
        self.is_loading()
    }

    /// The first load failed and there is no data.
    pub fn is_loading_error(&self) -> bool {
        self.is_error() && !self.entry().fields.has_data.get()
    }

    /// A refetch failed and the old data is still there.
    pub fn is_refetch_error(&self) -> bool {
        self.is_error() && self.entry().fields.has_data.get()
    }

    /// A load is running while there is already data or an error to show.
    pub fn is_refetching(&self) -> bool {
        self.is_fetching() && self.status_v5() != Status::Pending
    }

    pub fn is_paused(&self) -> bool {
        self.fetch_status() == FetchStatus::Paused
    }

    /// The data shown is placeholder data, not the cached value.
    pub fn is_placeholder_data(&self) -> bool {
        self.placeholder_active()
    }

    /// The data is older than its stale time, or was invalidated. Disabled queries are not stale.
    pub fn is_stale(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let entry = self.entry();
        entry.fields.data_version.get();
        entry.fields.has_data.get();
        entry.fields.invalidated.get();
        !entry
            .resource
            .with_untracked(|r| r.is_cache_fresh(now_ms()))
    }

    /// Whether the query is enabled. Tracked.
    pub fn is_enabled(&self) -> bool {
        self.enabled.get()
    }

    fn placeholder_active(&self) -> bool {
        let entry = self.entry();
        if entry.fields.has_data.get() || entry.fields.outcome.get() != Outcome::Pending {
            return false;
        }
        if self.placeholder.with_untracked(|p| p.is_some()) {
            return true;
        }
        self.keep_previous
            && self.previous.with_untracked(|p| *p).is_some_and(|prev| {
                prev.owner.is_live() && prev.resource.with_untracked(|r| r.data().is_some())
            })
    }

    /// Loads again now, ignoring freshness. One load at a time unless the
    /// options chose `RequestPolicy::LatestWins`.
    pub fn refetch(&self) {
        self.current.get_untracked().fetch(QueryFetchMode::Force);
    }

    /// Replaces the data, keeping the old value for [`Self::rollback`]. For
    /// optimistic updates.
    pub fn set_data(&self, data: T) {
        let entry = self.current.get_untracked();
        entry.resource.update(|r| r.set_data(data));
        entry.meta.update(|m| {
            m.outcome = Outcome::Success;
            m.data_updated_at = now_ms();
            m.data_update_count += 1;
            m.data_version += 1;
            m.invalidated = false;
        });
        query_client().touch();
    }

    /// Puts back the value [`Self::set_data`] replaced. False when there is none.
    pub fn rollback(&self) -> bool {
        let mut rolled = false;
        let entry = self.current.get_untracked();
        entry.resource.update(|r| rolled = r.rollback_to_previous());
        if rolled {
            entry.meta.update(|m| m.data_version += 1);
        }
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
    use_query_config(key, QueryConfig::from(options), fetcher)
}

/// [`use_query`] with a [`QueryConfig`], for options that name `T` or `E`.
pub fn use_query_config<T, E, F, Fut>(
    key: impl Into<QueryKey>,
    config: QueryConfig<T, E>,
    fetcher: F,
) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    let key = key.into();
    use_query_by_config(move || key.clone(), config, fetcher)
}

/// An observer with `subscribed(false)`. It reads the cache under the current
/// key, and never fetches, observes or refetches.
fn unsubscribed_query<T: 'static, E: 'static, K: Into<QueryKey>>(
    client: QueryClient,
    key: impl Fn() -> K + 'static,
    config: Rc<QueryConfig<T, E>>,
    fetcher: Fetcher<T, E>,
) -> Query<T, E> {
    let initial: QueryKey = untracked(|| key().into());
    let first_config = client.resolve_query_config(&initial, &config);
    let first = Entry::<T, E>::get_or_create(&client, &initial, &first_config);
    first.set_fetcher(fetcher.clone());
    first.hold();
    let current = signal_with(first, SignalEquals::Never);
    let held = Rc::new(Cell::new(first));
    let released = held.clone();
    effect(move || {
        let next_key: QueryKey = key().into();
        untracked(|| {
            if current.get_untracked().key() != next_key {
                let next_config = client.resolve_query_config(&next_key, &config);
                let next = Entry::<T, E>::get_or_create(&client, &next_key, &next_config);
                next.set_fetcher(fetcher.clone());
                next.hold();
                // The entry we leave loses its handle, so its gc starts if nothing else follows it.
                current.get_untracked().unhold();
                held.set(next);
                current.set(next);
            }
        });
    });
    // The entry's owner ends when this component goes and the cache no longer holds it.
    cleanup(move || released.get().unhold());
    Query {
        current,
        enabled: signal(true),
        placeholder: Tracked::new(None::<T>),
        previous: Tracked::new(None::<Entry<T, E>>),
        keep_previous: false,
        throw: None,
    }
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
    use_query_by_config(key, QueryConfig::from(options), fetcher)
}

/// [`use_query_by`] with a [`QueryConfig`].
pub fn use_query_by_config<T, E, K, F, Fut>(
    key: impl Fn() -> K + 'static,
    config: QueryConfig<T, E>,
    fetcher: F,
) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    K: Into<QueryKey>,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    observe_by(key, config, plain_fetcher(fetcher), true)
}

/// The observer behind every `use_query*` hook. With `autoload` false it loads
/// nothing on mount or on a key change; only [`Query::refetch`] and the other
/// explicit loads do.
pub(crate) fn observe_by<T, E, K>(
    key: impl Fn() -> K + 'static,
    config: QueryConfig<T, E>,
    fetcher: Fetcher<T, E>,
    autoload: bool,
) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    K: Into<QueryKey>,
{
    let client = query_client();
    let config = Rc::new(config);
    if !config.options.subscribed {
        return unsubscribed_query(client, key, config, fetcher);
    }
    let enabled_now =
        |cfg: &QueryConfig<T, E>| cfg.options.enabled.as_ref().is_none_or(|when| when());

    let initial: QueryKey = untracked(|| key().into());
    let first_config = client.resolve_query_config(&initial, &config);
    let first = Entry::<T, E>::get_or_create(&client, &initial, &first_config);
    first.set_fetcher(fetcher.clone());
    first.observe();
    let current = signal_with(first, SignalEquals::Never);
    let enabled = signal(untracked(|| enabled_now(&first_config)));
    let keep_previous = first_config.options.keep_previous_data;
    let placeholder = Tracked::new(None::<T>);
    let previous = Tracked::new(None::<Entry<T, E>>);
    let compute_placeholder = |cfg: &QueryConfig<T, E>, prev: Option<&T>| {
        cfg.placeholder_data.as_ref().and_then(|f| f(prev))
    };
    if first.resource.with_untracked(|r| r.data().is_none()) {
        let value = compute_placeholder(&first_config, None);
        placeholder.update(|p| *p = value);
    }

    let observed = Rc::new(Cell::new(first));
    let released = observed.clone();
    let mounted = stored(true);
    let effect_config = config.clone();

    effect(move || {
        let next_key: QueryKey = key().into();
        let next_config = client.resolve_query_config(&next_key, &effect_config);
        // Read outside `untracked`, so the effect re-runs when the condition's signals change.
        let on = enabled_now(&next_config);
        untracked(|| {
            let mounting = mounted.replace(false);
            let shown = current.get_untracked();
            enabled.set(on);
            let entry = if shown.key() == next_key {
                shown
            } else {
                let next = Entry::<T, E>::get_or_create(&client, &next_key, &next_config);
                next.set_fetcher(fetcher.clone());
                next.observe();
                if next.resource.with_untracked(|r| r.data().is_none()) {
                    let value = shown
                        .resource
                        .with_untracked(|r| compute_placeholder(&next_config, r.data()));
                    placeholder.update(|p| *p = value);
                } else {
                    placeholder.update(|p| *p = None);
                }
                // Placeholder and previous are written before `current`, so a reader that
                // re-runs on the key change sees both.
                previous.update(|p| *p = Some(shown));
                current.set(next);
                observed.set(next);
                shown.release();
                next
            };
            if !on {
                return;
            }
            let has_data = entry.resource.with_untracked(|r| r.data().is_some());
            // TanStack's shouldLoadOnMount: without data, load unless a retry
            // on mount is refused after an error. With data, refetchOnMount decides.
            let mode = if !autoload {
                None
            } else if !mounting {
                Some(QueryFetchMode::Normal)
            } else if !has_data {
                let refused = !next_config.options.retry_on_mount
                    && entry.meta.with_untracked(|m| m.outcome) == Outcome::Error;
                (!refused).then_some(QueryFetchMode::Normal)
            } else {
                match next_config.options.refetch_on_mount {
                    RefetchTrigger::Always => Some(QueryFetchMode::Force),
                    RefetchTrigger::IfStale => Some(QueryFetchMode::Normal),
                    RefetchTrigger::Never => None,
                }
            };
            if let Some(mode) = mode {
                entry.fetch(mode);
            }
        });
    });

    cleanup(move || released.get().release());

    let in_background = config.options.refetch_interval_in_background;
    if let Some(every) = config.options.refetch_interval {
        drop(set_interval(every, move || {
            if in_background || focus_manager().is_focused() {
                current.get_untracked().fetch(QueryFetchMode::Force);
            }
        }));
    }
    if let Some(next_delay) = config.options.refetch_interval_fn.clone() {
        drop(spawn(async move {
            while let Some(delay) = next_delay() {
                timer(delay).await;
                if in_background || focus_manager().is_focused() {
                    let entry = current.get_untracked();
                    entry.fetch(QueryFetchMode::Force);
                    // TanStack asks the function again after the result lands, so wait for it.
                    entry.wait_idle().await;
                }
            }
        }));
    }

    if config.options.refetch_on_foreground != RefetchTrigger::Never {
        let trigger = config.options.refetch_on_foreground;
        let was = stored(untracked(|| focus_manager().is_focused()));
        effect(move || {
            let now = focus_manager().is_focused();
            let before = was.replace(now);
            if now && !before {
                let mode = match trigger {
                    RefetchTrigger::Always => QueryFetchMode::Force,
                    _ => QueryFetchMode::Normal,
                };
                untracked(|| current.get_untracked().fetch(mode));
            }
        });
    }

    Query {
        current,
        enabled,
        placeholder,
        previous,
        keep_previous,
        throw: config.throw,
    }
}

/// [`use_query_with`] whose fetcher returns [`Fetched`]. The `Fetched` cache policy,
/// when set, replaces the entry's on success, so the server can say how long its answer stays fresh.
pub fn use_query_with_policy<T, E, F, Fut>(
    key: impl Into<QueryKey>,
    options: QueryOptions,
    fetcher: F,
) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<Fetched<T>, E>> + 'static,
{
    let key = key.into();
    observe_by(
        move || key.clone(),
        QueryConfig::from(options),
        policy_fetcher(fetcher),
        true,
    )
}

/// TanStack's manual observer: it follows `key` but loads nothing on mount or on a key
/// change. [`Query::refetch`] loads it, using `fetcher`.
pub fn use_query_manual<T, E, F, Fut>(key: impl Into<QueryKey>, fetcher: F) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    use_query_manual_opts(key, QueryOptions::default(), fetcher)
}

/// [`use_query_manual`] with options.
pub fn use_query_manual_opts<T, E, F, Fut>(
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
    observe_by(
        move || key.clone(),
        QueryConfig::from(options),
        plain_fetcher(fetcher),
        false,
    )
}

/// [`use_query`] for a fetcher that takes no [`QueryContext`]: it cannot read the key, the
/// cancellation signal or the client, so it loads what it was written to load.
pub fn use_query_unsignalled<T, E, F, Fut>(key: impl Into<QueryKey>, fetcher: F) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    use_query_unsignalled_opts(key, QueryOptions::default(), fetcher)
}

/// [`use_query_unsignalled`] with options.
pub fn use_query_unsignalled_opts<T, E, F, Fut>(
    key: impl Into<QueryKey>,
    options: QueryOptions,
    fetcher: F,
) -> Query<T, E>
where
    T: 'static,
    E: 'static,
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    use_query_with(key, options, move |_| fetcher())
}
