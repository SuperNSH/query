use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::task::{Poll, Waker};

use gpui_query::{MutationStatus, QueryError, QueryKey};
use nsh_core::JsonValue;
use nsh_core::prelude::*;

use crate::cache::{
    CacheEvent, MutationCache, MutationCacheEvent, MutationEvent, MutationListener, QueryCache,
    QueryClientConfig, QueryListener,
};
use crate::diagnostics::QueryDiagnostic;
use crate::mutation::{AnyMutation, MutationState};
use crate::options::{MutationDefaults, QueryConfig, QueryOptions};
use crate::query::{Entry, plain_fetcher};
use crate::status::{FetchStatus, Status};

/// One cached query, of any value type, as the client sees it.
pub(crate) trait AnyEntry {
    fn uid(&self) -> u64;
    fn state(&self) -> QueryState;
    /// Marks the data stale, and reloads it now when something observes it.
    fn invalidate(&self);
    fn cancel(&self);
    /// Back to its initial state, and reloads when observed.
    fn reset(&self);
    /// Loads again now, ignoring freshness.
    fn refetch(&self);
    fn dispose(&self);
    /// Whether an observer follows the entry, or a handle reads it. Such an entry's
    /// owner outlives its eviction from the cache.
    fn is_held(&self) -> bool;
    fn diagnostic(&self, now: u64) -> QueryDiagnostic;
    #[cfg_attr(not(feature = "persist"), allow(dead_code))]
    fn dehydrate(&self) -> Option<(JsonValue, u64)>;
    fn as_any(&self) -> &dyn Any;
}

type EntryId = (TypeId, QueryKey);

/// A snapshot of one cached query, readable without its value type.
#[derive(Clone, Debug, PartialEq)]
pub struct QueryState {
    pub key: QueryKey,
    pub status: Status,
    pub fetch_status: FetchStatus,
    pub data_updated_at: u64,
    pub error_updated_at: u64,
    pub data_update_count: u32,
    pub error_update_count: u32,
    pub failure_count: u32,
    pub is_invalidated: bool,
    pub is_stale: bool,
    pub observers: usize,
    pub has_data: bool,
}

/// Which observed queries a filter matches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueryType {
    #[default]
    All,
    /// Queries with at least one observer.
    Active,
    /// Queries with no observer.
    Inactive,
}

/// A test of a query's key, for [`QueryFilters::predicate`].
pub type KeyPredicate = Rc<dyn Fn(&QueryKey) -> bool>;

/// TanStack's query filters. A `From<&str>` or `From<QueryKey>` is a prefix match.
#[derive(Clone, Default)]
pub struct QueryFilters {
    pub query_key: Option<QueryKey>,
    /// Match the key exactly, not as a prefix.
    pub exact: bool,
    pub query_type: QueryType,
    pub stale: Option<bool>,
    pub fetch_status: Option<FetchStatus>,
    pub predicate: Option<KeyPredicate>,
}

impl QueryFilters {
    pub(crate) fn matches(&self, entry: &dyn AnyEntry) -> bool {
        self.matches_state(&entry.state())
    }

    /// Whether a query's snapshot matches these filters.
    pub(crate) fn matches_state(&self, state: &QueryState) -> bool {
        if let Some(key) = &self.query_key {
            let matched = if self.exact {
                state.key == *key
            } else {
                state.key.starts_with(key)
            };
            if !matched {
                return false;
            }
        }
        match self.query_type {
            QueryType::All => {}
            QueryType::Active if state.observers == 0 => return false,
            QueryType::Inactive if state.observers > 0 => return false,
            _ => {}
        }
        if self.stale.is_some_and(|stale| stale != state.is_stale) {
            return false;
        }
        if self
            .fetch_status
            .is_some_and(|status| status != state.fetch_status)
        {
            return false;
        }
        self.predicate.as_ref().is_none_or(|p| p(&state.key))
    }
}

impl From<&str> for QueryFilters {
    fn from(key: &str) -> Self {
        Self::from(QueryKey::from(key))
    }
}

impl From<QueryKey> for QueryFilters {
    fn from(key: QueryKey) -> Self {
        Self {
            query_key: Some(key),
            ..Self::default()
        }
    }
}

/// Which mutations a filter matches.
#[derive(Clone, Default)]
pub struct MutationFilters {
    pub mutation_key: Option<QueryKey>,
    pub status: Option<MutationStatus>,
}

/// TanStack's `matchQuery`: whether a query's state matches the filters.
pub fn match_query(filters: &QueryFilters, state: &QueryState) -> bool {
    filters.matches_state(state)
}

/// TanStack's `matchMutation`: whether a mutation's state matches the filters.
pub fn match_mutation(filters: &MutationFilters, state: &MutationState) -> bool {
    filters.matches(state)
}

/// TanStack's `hashKey`: a stable string for a key, as a JSON array of its parts.
/// Keys are compared by value in this crate, so the hash is for logs and keys
/// in other stores, not for the cache's identity.
pub fn hash_key(key: &QueryKey) -> String {
    let parts: Vec<String> = key
        .parts()
        .iter()
        .map(|part| {
            let escaped: String = part
                .chars()
                .flat_map(|c| match c {
                    '"' => vec!['\\', '"'],
                    '\\' => vec!['\\', '\\'],
                    c => vec![c],
                })
                .collect();
            format!("\"{escaped}\"")
        })
        .collect();
    format!("[{}]", parts.join(","))
}

impl std::fmt::Debug for QueryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QueryClient")
    }
}

impl MutationFilters {
    pub(crate) fn matches(&self, state: &MutationState) -> bool {
        if let Some(key) = &self.mutation_key
            && !state
                .mutation_key
                .as_ref()
                .is_some_and(|k| k.starts_with(key))
        {
            return false;
        }
        self.status.is_none_or(|status| status == state.status)
    }
}

impl From<QueryKey> for MutationFilters {
    fn from(key: QueryKey) -> Self {
        Self {
            mutation_key: Some(key),
            status: None,
        }
    }
}

#[derive(Default)]
pub(crate) struct ScopeState {
    busy: bool,
    waiters: VecDeque<Waker>,
}

#[derive(Default)]
pub(crate) struct Defaults {
    pub(crate) query: QueryOptions,
    pub(crate) query_prefixes: Vec<(QueryKey, QueryOptions)>,
    pub(crate) mutation_prefixes: Vec<(QueryKey, MutationDefaults)>,
}

/// Everything one surface's queries share. It lives in a global signal, so it
/// is per surface like the rest of NSH state.
pub(crate) struct Store {
    pub(crate) entries: RefCell<HashMap<EntryId, Rc<dyn AnyEntry>>>,
    pub(crate) mutations: RefCell<Vec<(u64, Rc<dyn AnyMutation>)>>,
    pub(crate) scopes: RefCell<HashMap<String, ScopeState>>,
    pub(crate) online_waiters: RefCell<Vec<Waker>>,
    pub(crate) hydrated: RefCell<Vec<(QueryKey, JsonValue, u64)>>,
    pub(crate) defaults: RefCell<Defaults>,
    pub(crate) mutation_ids: Cell<u64>,
    pub(crate) resumes: Cell<u64>,
    pub(crate) query_listeners: RefCell<Vec<(u64, QueryListener)>>,
    pub(crate) mutation_listeners: RefCell<Vec<(u64, MutationListener)>>,
    pub(crate) listener_ids: Cell<u64>,
    #[cfg_attr(not(feature = "persist"), allow(dead_code))]
    pub(crate) restoring: Signal<bool>,
    pub(crate) config: RefCell<QueryClientConfig>,
    /// Bumped on every change a count or a list reads, so those re-run.
    pub(crate) activity: Signal<u64>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            entries: RefCell::default(),
            mutations: RefCell::default(),
            scopes: RefCell::default(),
            online_waiters: RefCell::default(),
            hydrated: RefCell::default(),
            defaults: RefCell::default(),
            mutation_ids: Cell::new(0),
            resumes: Cell::new(0),
            query_listeners: RefCell::default(),
            mutation_listeners: RefCell::default(),
            listener_ids: Cell::new(0),
            restoring: signal(false),
            config: RefCell::default(),
            activity: signal(0),
        }
    }
}

/// The surface's query cache. Entries are keyed by value type and [`QueryKey`],
/// so one key can hold two types without colliding.
#[derive(Clone, Copy)]
pub struct QueryClient(pub(crate) Signal<Store>);

/// The client of the current surface. Every call returns the same cache.
pub fn query_client() -> QueryClient {
    QueryClient(global::<Store>())
}

pub(crate) fn prefix_filters(prefix: impl Into<QueryKey>) -> QueryFilters {
    QueryFilters::from(prefix.into())
}

impl QueryClient {
    pub(crate) fn lookup<T: Clone + 'static>(&self, id: &EntryId) -> Option<T> {
        self.0.with_untracked(|store| {
            store
                .entries
                .borrow()
                .get(id)
                .and_then(|e| e.as_any().downcast_ref::<T>().cloned())
        })
    }

    pub(crate) fn insert(&self, id: EntryId, entry: Rc<dyn AnyEntry>) {
        let key = entry.state().key;
        self.0.with_untracked(|store| {
            store.entries.borrow_mut().insert(id, entry);
        });
        self.touch();
        self.emit_query(CacheEvent::Added(key));
    }

    /// Whether an entry with this uid is still in the cache.
    pub(crate) fn is_registered(&self, uid: u64) -> bool {
        self.0
            .with_untracked(|store| store.entries.borrow().values().any(|e| e.uid() == uid))
    }

    /// Drops the entry for `id` if it is still `uid`, which a stale gc timer may not be.
    pub(crate) fn evict_uid(&self, id: &EntryId, uid: u64) {
        let removed = self.0.with_untracked(|store| {
            let mut entries = store.entries.borrow_mut();
            match entries.get(id) {
                Some(entry) if entry.uid() == uid => entries.remove(id),
                _ => None,
            }
        });
        // An entry that a handle still reads stays alive until that handle lets go.
        let key = removed.as_ref().map(|entry| entry.state().key);
        if let Some(entry) = removed
            && !entry.is_held()
        {
            entry.dispose();
        }
        self.touch();
        if let Some(key) = key {
            self.emit_query(CacheEvent::Removed(key));
        }
    }

    /// Records that a fetch, mutation or cache write changed something a count reads.
    pub(crate) fn touch(&self) {
        self.0
            .with_untracked(|store| store.activity.update(|v| *v = v.wrapping_add(1)));
    }

    /// The activity counter, tracked: a computation that reads it re-runs on every change.
    pub(crate) fn activity(&self) -> u64 {
        self.0.with(|store| store.activity.get())
    }

    pub(crate) fn matching(&self, filters: &QueryFilters) -> Vec<Rc<dyn AnyEntry>> {
        self.0.with_untracked(|store| {
            store
                .entries
                .borrow()
                .values()
                .filter(|e| filters.matches(e.as_ref()))
                .cloned()
                .collect()
        })
    }

    /// Marks every entry under `prefix` stale and reloads the observed ones.
    /// A load already running is replaced, so its older answer is ignored.
    pub fn invalidate(&self, prefix: impl Into<QueryKey>) {
        self.invalidate_queries(prefix_filters(prefix));
    }

    /// Stops the loads running under `prefix`. Data they already loaded is kept.
    pub fn cancel(&self, prefix: impl Into<QueryKey>) {
        self.cancel_queries(prefix_filters(prefix));
    }

    /// Drops every entry under `prefix`. Components still observing one keep it
    /// until they go, and a new observer starts from nothing.
    pub fn remove(&self, prefix: impl Into<QueryKey>) {
        self.remove_queries(prefix_filters(prefix));
    }

    /// Marks every matching query stale, and reloads the observed ones.
    pub fn invalidate_queries(&self, filters: impl Into<QueryFilters>) {
        for entry in self.matching(&filters.into()) {
            entry.invalidate();
        }
    }

    /// Stops the loads of every matching query. Their data is kept.
    pub fn cancel_queries(&self, filters: impl Into<QueryFilters>) {
        for entry in self.matching(&filters.into()) {
            entry.cancel();
        }
    }

    /// Loads every matching query again, ignoring freshness.
    pub fn refetch_queries(&self, filters: impl Into<QueryFilters>) {
        for entry in self.matching(&filters.into()) {
            entry.refetch();
        }
    }

    /// Returns every matching query to its initial state, and reloads the observed ones.
    pub fn reset_queries(&self, filters: impl Into<QueryFilters>) {
        for entry in self.matching(&filters.into()) {
            entry.reset();
        }
    }

    /// Drops every matching query. An observed one is dropped when its observer goes.
    pub fn remove_queries(&self, filters: impl Into<QueryFilters>) {
        let matched = self.matching(&filters.into());
        if matched.is_empty() {
            return;
        }
        let uids: Vec<u64> = matched.iter().map(|e| e.uid()).collect();
        let keys: Vec<QueryKey> = matched.iter().map(|e| e.state().key).collect();
        self.0.with_untracked(|store| {
            store
                .entries
                .borrow_mut()
                .retain(|_, e| !uids.contains(&e.uid()));
        });
        for entry in matched {
            if !entry.is_held() {
                entry.dispose();
            }
        }
        self.touch();
        for key in keys {
            self.emit_query(CacheEvent::Removed(key));
        }
    }

    /// Drops every query and mutation from the cache.
    pub fn clear(&self) {
        self.remove_queries(QueryFilters::default());
        self.0
            .with_untracked(|store| store.mutations.borrow_mut().clear());
        self.touch();
    }

    /// The snapshot of the query under exactly `key`, of any value type.
    pub fn get_query_state(&self, key: impl Into<QueryKey>) -> Option<QueryState> {
        let key = key.into();
        self.matching(&QueryFilters {
            query_key: Some(key.clone()),
            exact: true,
            ..QueryFilters::default()
        })
        .first()
        .map(|e| e.state())
    }

    /// How many matching queries are loading right now.
    pub fn is_fetching(&self, filters: impl Into<QueryFilters>) -> usize {
        self.matching(&filters.into())
            .iter()
            .filter(|e| e.state().fetch_status == FetchStatus::Fetching)
            .count()
    }

    /// How many matching mutations are running, paused or not.
    pub fn is_mutating(&self, filters: impl Into<MutationFilters>) -> usize {
        self.mutation_states(&filters.into())
            .iter()
            .filter(|s| s.status == MutationStatus::Loading)
            .count()
    }

    /// The cached value of `key`, untracked. `None` when nothing is cached.
    pub fn data<T: Clone + 'static, E: 'static>(&self, key: impl Into<QueryKey>) -> Option<T> {
        Entry::<T, E>::find(self, &key.into()).and_then(|entry| entry.data_untracked())
    }

    /// The cached value of `key` with the default error type.
    pub fn data_of<T: Clone + 'static>(&self, key: impl Into<QueryKey>) -> Option<T> {
        self.data::<T, QueryError>(key)
    }

    /// [`Self::data`] under TanStack's name.
    pub fn get_query_data<T: Clone + 'static, E: 'static>(
        &self,
        key: impl Into<QueryKey>,
    ) -> Option<T> {
        self.data::<T, E>(key)
    }

    /// The cached values of every matching query of this value and error type.
    pub fn get_queries_data<T: Clone + 'static, E: 'static>(
        &self,
        filters: impl Into<QueryFilters>,
    ) -> Vec<(QueryKey, T)> {
        self.matching(&filters.into())
            .into_iter()
            .filter_map(|entry| {
                let typed = entry.as_any().downcast_ref::<Entry<T, E>>()?;
                let data = typed.data_untracked()?;
                Some((entry.state().key, data))
            })
            .collect()
    }

    /// Calls `f` with the cached data of `key`, by reference, so no clone is needed. `None`
    /// when nothing is cached. A binding that calls it re-runs when that entry's data
    /// changes; a key with no entry yet is not watched, so use a query hook to wait for one.
    pub fn with_query_data<T: 'static, E: 'static, R>(
        &self,
        key: impl Into<QueryKey>,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        Entry::<T, E>::find(self, &key.into())?.with_data(f)
    }

    /// Writes `data` into the cache as a fresh result, creating the entry if needed.
    pub fn set_data<T: 'static, E: 'static>(&self, key: impl Into<QueryKey>, data: T) {
        let key = key.into();
        let config = self.resolve_query_config::<T, E>(&key, &QueryConfig::default());
        Entry::<T, E>::get_or_create(self, &key, &config).write(data, crate::now_ms(), true);
    }

    /// [`Self::set_data`] with the default error type.
    pub fn set_data_of<T: 'static>(&self, key: impl Into<QueryKey>, data: T) {
        self.set_data::<T, QueryError>(key, data);
    }

    /// [`Self::set_data`] under TanStack's name.
    pub fn set_query_data<T: 'static, E: 'static>(&self, key: impl Into<QueryKey>, data: T) {
        self.set_data::<T, E>(key, data);
    }

    /// TanStack's updater form: `updater` gets the current value and returns the
    /// new one. Returning `None` leaves the cache alone. False when nothing changed.
    pub fn update_query_data<T: Clone + 'static, E: 'static>(
        &self,
        key: impl Into<QueryKey>,
        updater: impl FnOnce(Option<&T>) -> Option<T>,
    ) -> bool {
        match Entry::<T, E>::find(self, &key.into()) {
            Some(entry) => entry.update_data(updater),
            None => false,
        }
    }

    /// [`Self::update_query_data`] over every matching query of this type.
    pub fn set_queries_data<T: Clone + 'static, E: 'static>(
        &self,
        filters: impl Into<QueryFilters>,
        updater: impl Fn(Option<&T>) -> Option<T>,
    ) {
        for entry in self.matching(&filters.into()) {
            if let Some(typed) = entry.as_any().downcast_ref::<Entry<T, E>>() {
                typed.update_data(&updater);
            }
        }
    }

    /// Loads `key` into the cache without an observer, unless it is still fresh.
    pub fn prefetch<T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: QueryOptions,
        fetcher: F,
    ) where
        T: 'static,
        E: 'static,
        F: Fn(crate::QueryContext) -> Fut + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + 'static,
    {
        let key = key.into();
        let config = self.resolve_query_config::<T, E>(&key, &QueryConfig::from(options));
        let entry = Entry::<T, E>::get_or_create(self, &key, &config);
        entry.set_fetcher(plain_fetcher(fetcher));
        entry.fetch(gpui_query::QueryFetchMode::Normal);
    }

    /// Sets the options every query inherits.
    pub fn set_default_options(&self, options: QueryOptions) {
        self.0
            .with_untracked(|s| s.defaults.borrow_mut().query = options);
    }

    pub fn get_default_options(&self) -> QueryOptions {
        self.0.with_untracked(|s| s.defaults.borrow().query.clone())
    }

    /// Sets the options of every query whose key starts with `prefix`.
    pub fn set_query_defaults(&self, prefix: impl Into<QueryKey>, options: QueryOptions) {
        let prefix = prefix.into();
        self.0.with_untracked(|s| {
            let mut defaults = s.defaults.borrow_mut();
            defaults.query_prefixes.retain(|(p, _)| *p != prefix);
            defaults.query_prefixes.push((prefix, options));
        });
    }

    pub fn get_query_defaults(&self, prefix: impl Into<QueryKey>) -> Option<QueryOptions> {
        let prefix = prefix.into();
        self.0.with_untracked(|s| {
            s.defaults
                .borrow()
                .query_prefixes
                .iter()
                .find(|(p, _)| *p == prefix)
                .map(|(_, o)| o.clone())
        })
    }

    /// Sets the defaults of every mutation whose key starts with `prefix`.
    pub fn set_mutation_defaults(&self, prefix: impl Into<QueryKey>, defaults: MutationDefaults) {
        let prefix = prefix.into();
        self.0.with_untracked(|s| {
            let mut all = s.defaults.borrow_mut();
            all.mutation_prefixes.retain(|(p, _)| *p != prefix);
            all.mutation_prefixes.push((prefix, defaults));
        });
    }

    pub fn get_mutation_defaults(&self, prefix: impl Into<QueryKey>) -> Option<MutationDefaults> {
        let prefix = prefix.into();
        self.0.with_untracked(|s| {
            s.defaults
                .borrow()
                .mutation_prefixes
                .iter()
                .find(|(p, _)| *p == prefix)
                .map(|(_, d)| d.clone())
        })
    }

    /// The options `key` gets: the call's options over the longest matching
    /// prefix default, over the client default.
    pub(crate) fn resolve_query_config<T: 'static, E: 'static>(
        &self,
        key: &QueryKey,
        config: &QueryConfig<T, E>,
    ) -> QueryConfig<T, E> {
        let base = self.0.with_untracked(|s| {
            let defaults = s.defaults.borrow();
            let mut base = defaults.query.clone();
            let mut best = 0;
            for (prefix, options) in &defaults.query_prefixes {
                if key.starts_with(prefix) && prefix.parts().len() >= best {
                    best = prefix.parts().len();
                    base = options.clone().over(&defaults.query);
                }
            }
            base
        });
        let mut resolved = config.clone();
        resolved.options = config.options.clone().over(&base);
        resolved
    }

    /// Parks a dehydrated value until a query with this key and a serializer is made.
    #[cfg_attr(not(feature = "persist"), allow(dead_code))]
    pub(crate) fn push_hydrated(&self, key: QueryKey, value: JsonValue, updated_at: u64) {
        self.0.with_untracked(|s| {
            let mut hydrated = s.hydrated.borrow_mut();
            hydrated.retain(|(k, _, _)| *k != key);
            hydrated.push((key, value, updated_at));
        });
    }

    pub(crate) fn take_hydrated(&self, key: &QueryKey) -> Option<(JsonValue, u64)> {
        self.0.with_untracked(|s| {
            let mut hydrated = s.hydrated.borrow_mut();
            let index = hydrated.iter().position(|(k, _, _)| k == key)?;
            let (_, value, at) = hydrated.remove(index);
            Some((value, at))
        })
    }

    /// Every cached query whose value persists, as its key, value and update time.
    #[cfg_attr(not(feature = "persist"), allow(dead_code))]
    pub(crate) fn dehydrated(&self) -> Vec<(QueryKey, JsonValue, u64)> {
        self.0.with_untracked(|s| {
            s.entries
                .borrow()
                .values()
                .filter_map(|e| {
                    let (value, at) = e.dehydrate()?;
                    Some((e.state().key, value, at))
                })
                .collect()
        })
    }

    /// The defaults of the longest mutation-key prefix that matches `key`.
    pub(crate) fn mutation_defaults_for(&self, key: &QueryKey) -> Option<MutationDefaults> {
        self.0.with_untracked(|s| {
            let defaults = s.defaults.borrow();
            defaults
                .mutation_prefixes
                .iter()
                .filter(|(prefix, _)| key.starts_with(prefix))
                .max_by_key(|(prefix, _)| prefix.parts().len())
                .map(|(_, d)| d.clone())
        })
    }

    pub(crate) fn add_online_waiter(&self, waker: Waker) {
        self.0
            .with_untracked(|s| s.online_waiters.borrow_mut().push(waker));
    }

    /// Lets paused fetches and mutations run again. Going online already does
    /// this; call it after the app has its own signal that the link is back.
    pub fn resume_paused_mutations(&self) {
        self.0
            .with_untracked(|s| s.resumes.set(s.resumes.get() + 1));
        self.wake_online_waiters();
    }

    pub(crate) fn resume_count(&self) -> u64 {
        self.0.with_untracked(|s| s.resumes.get())
    }

    pub(crate) fn wake_online_waiters(&self) {
        let waiters = self
            .0
            .with_untracked(|s| std::mem::take(&mut *s.online_waiters.borrow_mut()));
        for waker in waiters {
            waker.wake();
        }
        self.touch();
    }

    /// Lets the next mutation of `scope` run, or makes this one wait its turn.
    pub(crate) fn poll_scope(&self, scope: &str, waker: &Waker) -> Poll<()> {
        self.0.with_untracked(|s| {
            let mut scopes = s.scopes.borrow_mut();
            let state = scopes.entry(scope.to_owned()).or_default();
            if state.busy {
                state.waiters.push_back(waker.clone());
                Poll::Pending
            } else {
                state.busy = true;
                Poll::Ready(())
            }
        })
    }

    pub(crate) fn release_scope(&self, scope: &str) {
        let next = self.0.with_untracked(|s| {
            let mut scopes = s.scopes.borrow_mut();
            scopes.get_mut(scope).and_then(|state| {
                state.busy = false;
                state.waiters.pop_front()
            })
        });
        if let Some(waker) = next {
            waker.wake();
        }
    }

    pub(crate) fn register_mutation(&self, id: u64, mutation: Rc<dyn AnyMutation>) {
        let state = mutation.state();
        self.0
            .with_untracked(|s| s.mutations.borrow_mut().push((id, mutation)));
        self.touch();
        self.emit_mutation(MutationCacheEvent::Added(state));
    }

    pub(crate) fn unregister_mutation(&self, id: u64) {
        let removed = self.0.with_untracked(|s| {
            let mut all = s.mutations.borrow_mut();
            let state = all.iter().find(|(i, _)| *i == id).map(|(_, m)| m.state());
            all.retain(|(i, _)| *i != id);
            state
        });
        self.touch();
        if let Some(state) = removed {
            self.emit_mutation(MutationCacheEvent::Removed(state));
        }
    }

    /// Removes the settled mutations that match `filters`. A call in flight stays.
    pub(crate) fn remove_settled_mutations(&self, filters: MutationFilters) {
        let removed: Vec<MutationState> = self.0.with_untracked(|s| {
            let mut all = s.mutations.borrow_mut();
            let mut removed = Vec::new();
            all.retain(|(_, m)| {
                let state = m.state();
                let drop = filters.matches(&state) && state.status != MutationStatus::Loading;
                if drop {
                    removed.push(state);
                }
                !drop
            });
            removed
        });
        self.touch();
        for state in removed {
            self.emit_mutation(MutationCacheEvent::Removed(state));
        }
    }

    fn emit_query(&self, event: CacheEvent) {
        let listeners: Vec<QueryListener> = self.0.with_untracked(|s| {
            s.query_listeners
                .borrow()
                .iter()
                .map(|(_, f)| f.clone())
                .collect()
        });
        for listener in listeners {
            listener(&event);
        }
    }

    fn emit_mutation(&self, event: MutationCacheEvent) {
        let listeners: Vec<MutationListener> = self.0.with_untracked(|s| {
            s.mutation_listeners
                .borrow()
                .iter()
                .map(|(_, f)| f.clone())
                .collect()
        });
        for listener in listeners {
            listener(&event);
        }
    }

    pub(crate) fn add_query_listener(&self, f: Rc<dyn Fn(&CacheEvent)>) -> u64 {
        self.0.with_untracked(|s| {
            let id = s.listener_ids.get() + 1;
            s.listener_ids.set(id);
            s.query_listeners.borrow_mut().push((id, f));
            id
        })
    }

    pub(crate) fn remove_query_listener(&self, id: u64) {
        self.0
            .with_untracked(|s| s.query_listeners.borrow_mut().retain(|(i, _)| *i != id));
    }

    pub(crate) fn add_mutation_listener(&self, f: Rc<dyn Fn(&MutationCacheEvent)>) -> u64 {
        self.0.with_untracked(|s| {
            let id = s.listener_ids.get() + 1;
            s.listener_ids.set(id);
            s.mutation_listeners.borrow_mut().push((id, f));
            id
        })
    }

    pub(crate) fn remove_mutation_listener(&self, id: u64) {
        self.0
            .with_untracked(|s| s.mutation_listeners.borrow_mut().retain(|(i, _)| *i != id));
    }

    /// Fires the query-cache callbacks for a load that just settled.
    pub(crate) fn notify_query(&self, state: &QueryState, success: bool) {
        let callbacks = self
            .0
            .with_untracked(|s| s.config.borrow().query_cache.clone());
        if success {
            if let Some(f) = &callbacks.on_success {
                f(state);
            }
        } else if let Some(f) = &callbacks.on_error {
            f(state);
        }
        if let Some(f) = &callbacks.on_settled {
            f(state);
        }
    }

    /// Fires the mutation-cache callbacks for one point of a call.
    pub(crate) fn notify_mutation(&self, state: &MutationState, event: MutationEvent) {
        let callbacks = self
            .0
            .with_untracked(|s| s.config.borrow().mutation_cache.clone());
        let callback = match event {
            MutationEvent::Mutate => &callbacks.on_mutate,
            MutationEvent::Success => &callbacks.on_success,
            MutationEvent::Error => &callbacks.on_error,
            MutationEvent::Settled => &callbacks.on_settled,
        };
        if let Some(f) = callback {
            f(state);
        }
    }

    /// Sets the cache-wide callbacks. They replace the previous ones.
    pub fn set_config(&self, config: QueryClientConfig) {
        self.0.with_untracked(|s| *s.config.borrow_mut() = config);
    }

    /// TanStack's `getQueryCache()`.
    pub fn get_query_cache(&self) -> QueryCache {
        QueryCache::new(*self)
    }

    /// TanStack's `getMutationCache()`.
    pub fn get_mutation_cache(&self) -> MutationCache {
        MutationCache::new(*self)
    }

    /// Whether a restore is in progress, as a signal.
    #[cfg_attr(not(feature = "persist"), allow(dead_code))]
    pub(crate) fn restoring(&self) -> Signal<bool> {
        self.0.with_untracked(|s| s.restoring)
    }

    /// The states of every cached query that matches `filters`, in no set order.
    pub fn query_states(&self, filters: impl Into<QueryFilters>) -> Vec<QueryState> {
        let filters = filters.into();
        self.matching(&filters).iter().map(|e| e.state()).collect()
    }

    pub(crate) fn next_mutation_id(&self) -> u64 {
        self.0.with_untracked(|s| {
            let id = s.mutation_ids.get() + 1;
            s.mutation_ids.set(id);
            id
        })
    }

    /// The states of the cached mutations that match `filters`, oldest first.
    pub fn mutation_states(&self, filters: &MutationFilters) -> Vec<MutationState> {
        let mut states: Vec<MutationState> = self.0.with_untracked(|s| {
            s.mutations
                .borrow()
                .iter()
                .map(|(_, m)| m.state())
                .filter(|state| filters.matches(state))
                .collect()
        });
        states.sort_by_key(|s| s.submitted_at);
        states
    }
}
