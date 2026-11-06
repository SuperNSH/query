//! Async state for Native Shell Rust apps: `use_query`, `use_mutation` and
//! `use_infinite_query` over the GPUI-free state machine of `gpui-query`.
//! Results are signals, so a binding that reads one updates by itself.

mod client;
mod infinite;
mod mutation;
mod options;
mod query;
mod tracked;

pub use client::{QueryClient, query_client};
pub use gpui_query::{
    CachePolicy, MutationStatus, QueryError, QueryErrorKind, QueryKey, QuerySignal, QueryStatus,
    RefetchTrigger, RequestPolicy, RetryPolicy,
};
pub use infinite::{InfiniteQuery, InfiniteQueryOptions, use_infinite_query};
pub use mutation::{Mutation, MutationOptions, use_mutation, use_mutation_with};
pub use options::{QueryContext, QueryOptions};
pub use query::{Query, use_query, use_query_by, use_query_with};

pub(crate) fn now_ms() -> u64 {
    nsh_core::time::unix_ms() as u64
}
