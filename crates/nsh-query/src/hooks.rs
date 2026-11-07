use std::future::Future;
use std::rc::Rc;

use gpui_query::QueryKey;
use nsh_core::prelude::*;

use crate::client::{MutationFilters, QueryFilters, query_client};
use crate::mutation::MutationState;
use crate::options::{QueryContext, QueryOptions};
use crate::query::{Query, use_query_with};

/// Observes several queries of one value type, each with its own key and options.
/// They share `fetcher`, which tells them apart by [`QueryContext::key`].
pub fn use_queries<T, E, K, F, Fut>(queries: Vec<(K, QueryOptions)>, fetcher: F) -> Vec<Query<T, E>>
where
    K: Into<QueryKey>,
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    let fetcher = Rc::new(fetcher);
    queries
        .into_iter()
        .map(|(key, options)| {
            let fetcher = fetcher.clone();
            use_query_with(key, options, move |context| (fetcher)(context))
        })
        .collect()
}

/// [`use_queries`] with a `combine` that turns the queries into one value. The
/// value is a memo, so it re-runs only when a query it reads changes.
pub fn use_queries_combined<T, E, K, F, Fut, R>(
    queries: Vec<(K, QueryOptions)>,
    fetcher: F,
    combine: impl Fn(&[Query<T, E>]) -> R + 'static,
) -> Memo<R>
where
    K: Into<QueryKey>,
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    R: PartialEq + 'static,
{
    let handles = use_queries(queries, fetcher);
    computed(move || combine(&handles))
}

/// TanStack's `select` as a hook: observes `key` and returns a memo of `select` over its data.
/// A binding on the memo re-runs only when the selected value changes, not when the data does.
pub fn use_query_select<T, E, U, F, Fut>(
    key: impl Into<QueryKey>,
    options: QueryOptions,
    fetcher: F,
    select: impl Fn(&T) -> U + 'static,
) -> Memo<Option<U>>
where
    T: 'static,
    E: 'static,
    U: PartialEq + 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    use_query_with(key, options, fetcher).select(select)
}

/// TanStack's `usePrefetchQuery`: loads `key` into the cache when the component
/// is built, unless it is still fresh. It does not observe the query.
pub fn use_prefetch_query<T, E, F, Fut>(key: impl Into<QueryKey>, options: QueryOptions, fetcher: F)
where
    T: 'static,
    E: 'static,
    F: Fn(QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    query_client().prefetch(key, options, fetcher);
}

/// TanStack's `usePrefetchInfiniteQuery`: loads the first page into the cache when
/// the component is built, unless one is already loaded.
pub fn use_prefetch_infinite_query<P, T, E, F, Fut>(
    key: impl Into<QueryKey>,
    options: crate::infinite::InfiniteQueryOptions<P, T>,
    fetch_page: F,
) where
    P: Clone + 'static,
    T: 'static,
    E: 'static,
    F: Fn(P, QueryContext) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    query_client().prefetch_infinite_query(key, options, fetch_page);
}

/// How many matching queries are loading. Updates as loads start and end.
pub fn use_is_fetching(filters: impl Into<QueryFilters>) -> Memo<usize> {
    let filters = filters.into();
    let client = query_client();
    computed(move || {
        client.activity();
        client.is_fetching(filters.clone())
    })
}

/// How many matching mutations are running. Updates as calls start and end.
pub fn use_is_mutating(filters: impl Into<MutationFilters>) -> Memo<usize> {
    let filters = filters.into();
    let client = query_client();
    computed(move || {
        client.activity();
        client.is_mutating(filters.clone())
    })
}

/// The mutations matching `filters`, each mapped by `select`, oldest first.
pub fn use_mutation_state<R>(
    filters: impl Into<MutationFilters>,
    select: impl Fn(&MutationState) -> R + 'static,
) -> Memo<Vec<R>>
where
    R: PartialEq + 'static,
{
    let filters = filters.into();
    let client = query_client();
    computed(move || {
        client.activity();
        client
            .mutation_states(&filters)
            .iter()
            .map(&select)
            .collect()
    })
}

/// TanStack's `QueryErrorResetBoundary`: [`Self::reset`] reloads the queries that failed,
/// so an error boundary can try its view again.
#[derive(Clone, Copy)]
pub struct QueryErrorResetBoundary {
    client: crate::client::QueryClient,
}

impl QueryErrorResetBoundary {
    /// Reloads every query whose last load failed.
    pub fn reset(&self) {
        let failed: Vec<QueryKey> = self
            .client
            .query_states(QueryFilters::default())
            .into_iter()
            .filter(|state| state.status == crate::status::Status::Error)
            .map(|state| state.key)
            .collect();
        for key in failed {
            self.client.refetch_queries(QueryFilters {
                query_key: Some(key),
                exact: true,
                ..QueryFilters::default()
            });
        }
    }
}

/// TanStack's `useQueryErrorResetBoundary`: the reset handle for an error boundary.
pub fn use_query_error_reset_boundary() -> QueryErrorResetBoundary {
    QueryErrorResetBoundary {
        client: query_client(),
    }
}
