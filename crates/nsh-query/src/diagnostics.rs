//! gpui-query's diagnostics: a snapshot of every cached query and mutation, for a debug screen.

use gpui_query::{MutationStatus, QueryStatus};
use nsh_core::prelude::*;

use crate::client::{QueryClient, QueryFilters, query_client};

/// One cached query, as the diagnostics report it.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct QueryDiagnostic {
    /// The key's parts joined with `::`, for example `users::42::posts`.
    pub key: String,
    /// The core state machine's status, as [`crate::Query::status`] reports it.
    pub status: QueryStatus,
    /// The cache policy as text, from its `Display`, for example `No cache`.
    pub cache_policy: String,
    /// How old the cached data is, as of the last change to the cache. `None` without data.
    pub cache_age_ms: Option<u64>,
    /// How many times the cached data was served without a load.
    pub cache_hits: u64,
    /// Retries in the current load.
    pub retry_count: u32,
}

/// One cached mutation, as the diagnostics report it.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct MutationDiagnostic {
    /// The mutation key's parts joined with `::`, if the mutation has one.
    pub key: Option<String>,
    /// The mutation's status.
    pub status: MutationStatus,
    /// Retries in the latest call.
    pub retry_count: u32,
}

/// Every cached query and mutation, with their counts.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ClientDiagnostic {
    /// How many queries are cached, infinite ones included.
    pub query_count: usize,
    /// How many mutations are cached.
    pub mutation_count: usize,
    /// One entry per cached query, sorted by key.
    pub queries: Vec<QueryDiagnostic>,
    /// One entry per cached mutation, oldest first.
    pub mutations: Vec<MutationDiagnostic>,
}

impl QueryClient {
    /// A snapshot of the cache. It is untracked, so use [`use_diagnostics`] in a binding that
    /// must follow the cache.
    pub fn diagnostics(&self) -> ClientDiagnostic {
        let now = crate::now_ms();
        let mut queries: Vec<QueryDiagnostic> = self
            .matching(&QueryFilters::default())
            .iter()
            .map(|entry| entry.diagnostic(now))
            .collect();
        queries.sort_by(|a, b| a.key.cmp(&b.key));
        let mutations: Vec<MutationDiagnostic> = self.0.with_untracked(|store| {
            store
                .mutations
                .borrow()
                .iter()
                .map(|(_, mutation)| mutation.diagnostic())
                .collect()
        });
        ClientDiagnostic {
            query_count: queries.len(),
            mutation_count: mutations.len(),
            queries,
            mutations,
        }
    }
}

/// [`QueryClient::diagnostics`] as a memo, so a binding on it re-runs when the cache changes.
pub fn use_diagnostics() -> Memo<ClientDiagnostic> {
    let client = query_client();
    computed(move || {
        client.activity();
        client.diagnostics()
    })
}
