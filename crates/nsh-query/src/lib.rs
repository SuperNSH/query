//! Async state for Native Shell Rust apps, in TanStack Query's shape: queries,
//! mutations and infinite queries over the GPUI-free state machine of
//! `gpui-query`. Results are signals, so a binding that reads one updates by itself.

mod cache;
mod client;
mod diagnostics;
mod hooks;
mod infinite;
mod managers;
mod mutation;
mod options;
#[cfg(feature = "persist")]
mod persist;
mod query;
mod status;
mod tracked;

pub use cache::{
    CacheEvent, CacheSubscription, MutationCache, MutationCacheCallbacks, MutationCacheEvent,
    QueryCache, QueryCacheCallbacks, QueryClientConfig,
};
pub use client::{
    MutationFilters, QueryClient, QueryFilters, QueryState, QueryType, hash_key, match_mutation,
    match_query, query_client,
};
pub use diagnostics::{ClientDiagnostic, MutationDiagnostic, QueryDiagnostic, use_diagnostics};
pub use gpui_query::{
    CachePolicy, Fetched, MutationStatus, NetworkMode, QueryError, QueryErrorKind, QueryKey,
    QuerySignal, QueryStatus, RefetchTrigger, RequestPolicy, RetryPolicy,
};
pub use hooks::{
    QueryErrorResetBoundary, use_is_fetching, use_is_mutating, use_mutation_state,
    use_prefetch_infinite_query, use_prefetch_query, use_queries, use_queries_combined,
    use_query_error_reset_boundary, use_query_select,
};
pub use infinite::{InfiniteQuery, InfiniteQueryOptions, use_infinite_query};
pub use managers::{FocusManager, OnlineManager, focus_manager, online_manager};
pub use mutation::{
    MutateOptions, Mutation, MutationOptions, MutationState, use_mutation, use_mutation_with,
    use_mutation_with_context,
};
pub use options::{MutationDefaults, PageDirection, QueryConfig, QueryContext, QueryOptions};
pub use query::{
    FetchQueryError, Query, use_query, use_query_by, use_query_by_config, use_query_config,
    use_query_manual, use_query_manual_opts, use_query_unsignalled, use_query_unsignalled_opts,
    use_query_with, use_query_with_policy,
};
pub use status::{FetchStatus, Status};

#[cfg(feature = "persist")]
pub use persist::{
    DehydrateOptions, DehydratedMutation, DehydratedQuery, DehydratedState, HydrateOptions,
    PersistOptions, dehydrate, dehydrate_with, hydrate, hydrate_with, persist_to_storage,
    persist_to_storage_with, restore_from_storage, restore_from_storage_with, use_is_restoring,
};

/// The HTTP cache of `gpui-query-http`, behind the `http-cache` feature.
#[cfg(feature = "http-cache")]
pub use gpui_query_http::{HttpBackend, HttpCache};

pub(crate) fn now_ms() -> u64 {
    nsh_core::time::unix_ms() as u64
}
