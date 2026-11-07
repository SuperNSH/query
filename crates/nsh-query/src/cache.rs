//! TanStack's `QueryCache` and `MutationCache`: finding, listing, subscribing to and
//! removing cached queries and mutations, and the client config that sets the callbacks
//! fired for every query and every mutation.

use std::rc::Rc;

use gpui_query::QueryKey;

use crate::client::{MutationFilters, QueryClient, QueryFilters, QueryState};
use crate::mutation::MutationState;

/// What a query-cache subscriber hears: a query was added to the cache or removed from it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CacheEvent {
    Added(QueryKey),
    Removed(QueryKey),
}

/// What a mutation-cache subscriber hears: a mutation was added or removed.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum MutationCacheEvent {
    Added(MutationState),
    Removed(MutationState),
}

/// Which mutation-cache callback is firing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MutationEvent {
    Mutate,
    Success,
    Error,
    Settled,
}

pub(crate) type QueryCallback = Rc<dyn Fn(&QueryState)>;

/// A query-cache subscriber.
pub(crate) type QueryListener = Rc<dyn Fn(&CacheEvent)>;

/// A mutation-cache subscriber.
pub(crate) type MutationListener = Rc<dyn Fn(&MutationCacheEvent)>;
pub(crate) type MutationCallback = Rc<dyn Fn(&MutationState)>;

/// Callbacks for every query. They run after each load settles, and get the query's state.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct QueryCacheCallbacks {
    /// Runs after a load ends in an error.
    pub on_error: Option<QueryCallback>,
    /// Runs after a load succeeds.
    pub on_success: Option<QueryCallback>,
    /// Runs after a load ends, last.
    pub on_settled: Option<QueryCallback>,
}

impl QueryCacheCallbacks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_error(mut self, f: impl Fn(&QueryState) + 'static) -> Self {
        self.on_error = Some(Rc::new(f));
        self
    }

    pub fn on_success(mut self, f: impl Fn(&QueryState) + 'static) -> Self {
        self.on_success = Some(Rc::new(f));
        self
    }

    pub fn on_settled(mut self, f: impl Fn(&QueryState) + 'static) -> Self {
        self.on_settled = Some(Rc::new(f));
        self
    }
}

/// Callbacks for every mutation. They run at the start, after success or error, and last.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct MutationCacheCallbacks {
    /// Runs when a call starts.
    pub on_mutate: Option<MutationCallback>,
    /// Runs after a call fails.
    pub on_error: Option<MutationCallback>,
    /// Runs after a call succeeds.
    pub on_success: Option<MutationCallback>,
    /// Runs after a call settles, last.
    pub on_settled: Option<MutationCallback>,
}

impl MutationCacheCallbacks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_mutate(mut self, f: impl Fn(&MutationState) + 'static) -> Self {
        self.on_mutate = Some(Rc::new(f));
        self
    }

    pub fn on_error(mut self, f: impl Fn(&MutationState) + 'static) -> Self {
        self.on_error = Some(Rc::new(f));
        self
    }

    pub fn on_success(mut self, f: impl Fn(&MutationState) + 'static) -> Self {
        self.on_success = Some(Rc::new(f));
        self
    }

    pub fn on_settled(mut self, f: impl Fn(&MutationState) + 'static) -> Self {
        self.on_settled = Some(Rc::new(f));
        self
    }
}

/// The client's configuration: the cache-wide callbacks. Set it with
/// [`QueryClient::set_config`].
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct QueryClientConfig {
    /// Callbacks for the query cache.
    pub query_cache: QueryCacheCallbacks,
    /// Callbacks for the mutation cache.
    pub mutation_cache: MutationCacheCallbacks,
}

impl QueryClientConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn query_cache(mut self, callbacks: QueryCacheCallbacks) -> Self {
        self.query_cache = callbacks;
        self
    }

    pub fn mutation_cache(mut self, callbacks: MutationCacheCallbacks) -> Self {
        self.mutation_cache = callbacks;
        self
    }
}

/// A subscription to a cache. [`Self::stop`] ends it.
#[must_use = "a subscription runs until it is stopped"]
pub struct CacheSubscription {
    client: QueryClient,
    id: u64,
    query: bool,
}

impl CacheSubscription {
    /// Ends the subscription. The callback hears nothing after this.
    pub fn stop(self) {
        if self.query {
            self.client.remove_query_listener(self.id);
        } else {
            self.client.remove_mutation_listener(self.id);
        }
    }
}

/// TanStack's `QueryCache`, from [`QueryClient::get_query_cache`].
#[derive(Clone, Copy)]
pub struct QueryCache {
    client: QueryClient,
}

impl QueryCache {
    pub(crate) fn new(client: QueryClient) -> Self {
        Self { client }
    }

    /// The state of the query under exactly `key`, if it is cached.
    pub fn find(&self, key: impl Into<QueryKey>) -> Option<QueryState> {
        self.client.get_query_state(key)
    }

    /// The states of every cached query that matches `filters`.
    pub fn find_all(&self, filters: impl Into<QueryFilters>) -> Vec<QueryState> {
        self.client.query_states(filters)
    }

    /// The states of every cached query.
    pub fn get_all(&self) -> Vec<QueryState> {
        self.client.query_states(QueryFilters::default())
    }

    /// Calls `f` when a query is added to or removed from the cache.
    pub fn subscribe(&self, f: impl Fn(&CacheEvent) + 'static) -> CacheSubscription {
        CacheSubscription {
            client: self.client,
            id: self.client.add_query_listener(Rc::new(f)),
            query: true,
        }
    }

    /// Removes every query. Queries an observer still holds are removed when it lets go.
    pub fn clear(&self) {
        self.client.remove_queries(QueryFilters::default());
    }

    /// Removes the query under exactly `key`.
    pub fn remove(&self, key: impl Into<QueryKey>) {
        self.client.remove_queries(QueryFilters {
            query_key: Some(key.into()),
            exact: true,
            ..QueryFilters::default()
        });
    }
}

/// TanStack's `MutationCache`, from [`QueryClient::get_mutation_cache`].
#[derive(Clone, Copy)]
pub struct MutationCache {
    client: QueryClient,
}

impl MutationCache {
    pub(crate) fn new(client: QueryClient) -> Self {
        Self { client }
    }

    /// The first mutation that matches `filters`, oldest first.
    pub fn find(&self, filters: impl Into<MutationFilters>) -> Option<MutationState> {
        self.client
            .mutation_states(&filters.into())
            .into_iter()
            .next()
    }

    /// Every mutation that matches `filters`, oldest first.
    pub fn find_all(&self, filters: impl Into<MutationFilters>) -> Vec<MutationState> {
        self.client.mutation_states(&filters.into())
    }

    /// Every cached mutation, oldest first.
    pub fn get_all(&self) -> Vec<MutationState> {
        self.client.mutation_states(&MutationFilters::default())
    }

    /// Calls `f` when a mutation is added to or removed from the cache.
    pub fn subscribe(&self, f: impl Fn(&MutationCacheEvent) + 'static) -> CacheSubscription {
        CacheSubscription {
            client: self.client,
            id: self.client.add_mutation_listener(Rc::new(f)),
            query: false,
        }
    }

    /// Removes every mutation whose call has settled. Calls in flight stay.
    pub fn clear(&self) {
        self.client
            .remove_settled_mutations(MutationFilters::default());
    }

    /// Removes the settled mutations that match `filters`.
    pub fn remove(&self, filters: impl Into<MutationFilters>) {
        self.client.remove_settled_mutations(filters.into());
    }
}
