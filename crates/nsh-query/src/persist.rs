//! Saving the client's persisted queries to NSH storage, and restoring them. Only queries
//! made with `QueryConfig::persist` carry a value; the options choose which queries and
//! mutations are saved, and what a restore accepts.

use std::rc::Rc;
use std::time::Duration;

use gpui_query::QueryKey;
use nsh_core::prelude::*;
use nsh_core::storage::{Storage, StorageFull};
use serde_json::{Map, Value, json};

use crate::client::{MutationFilters, QueryClient, QueryFilters, QueryState, query_client};
use crate::mutation::MutationState;
use crate::options::QueryOptions;
use crate::status::Status;

/// One saved query: its key, its value as JSON, and when the value loaded.
#[derive(Clone, Debug, PartialEq)]
pub struct DehydratedQuery {
    pub key: QueryKey,
    pub value: Value,
    pub updated_at: u64,
}

/// One saved mutation, as information: a restore does not start mutations again, since a
/// mutation belongs to the component that made it.
#[derive(Clone, Debug, PartialEq)]
pub struct DehydratedMutation {
    pub mutation_key: Option<QueryKey>,
    pub submitted_at: u64,
}

/// The saved queries and mutations of a client.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DehydratedState {
    pub queries: Vec<DehydratedQuery>,
    pub mutations: Vec<DehydratedMutation>,
}

/// TanStack's `dehydrate` options. The defaults save successful queries and paused mutations,
/// as TanStack does.
#[non_exhaustive]
#[derive(Clone)]
pub struct DehydrateOptions {
    /// Whether a query is saved.
    pub should_dehydrate_query: Rc<dyn Fn(&QueryState) -> bool>,
    /// Whether a mutation is listed in the saved state.
    pub should_dehydrate_mutation: Rc<dyn Fn(&MutationState) -> bool>,
}

impl Default for DehydrateOptions {
    fn default() -> Self {
        Self {
            should_dehydrate_query: Rc::new(|state: &QueryState| state.status == Status::Success),
            should_dehydrate_mutation: Rc::new(|state: &MutationState| state.is_paused),
        }
    }
}

impl DehydrateOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn should_dehydrate_query(mut self, f: impl Fn(&QueryState) -> bool + 'static) -> Self {
        self.should_dehydrate_query = Rc::new(f);
        self
    }

    pub fn should_dehydrate_mutation(
        mut self,
        f: impl Fn(&MutationState) -> bool + 'static,
    ) -> Self {
        self.should_dehydrate_mutation = Rc::new(f);
        self
    }
}

/// TanStack's `hydrate` options: the defaults that hydrated queries get.
#[non_exhaustive]
#[derive(Clone, Default)]
pub struct HydrateOptions {
    /// Options applied to every hydrated query, as a default under its key.
    pub query_defaults: Option<QueryOptions>,
}

impl HydrateOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn query_defaults(mut self, options: QueryOptions) -> Self {
        self.query_defaults = Some(options);
        self
    }
}

/// TanStack's persist options for a restore: the saved cache is discarded when it is older
/// than `max_age`, or when its buster differs from this one.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PersistOptions {
    /// How old a saved cache may be. TanStack's default is 24 hours.
    pub max_age: Duration,
    /// A string that, when changed, discards every saved cache.
    pub buster: String,
}

impl Default for PersistOptions {
    fn default() -> Self {
        Self {
            max_age: Duration::from_secs(24 * 60 * 60),
            buster: String::new(),
        }
    }
}

impl PersistOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    pub fn buster(mut self, buster: impl Into<String>) -> Self {
        self.buster = buster.into();
        self
    }
}

/// Snapshots the queries and mutations that `options` select.
pub fn dehydrate_with(client: &QueryClient, options: &DehydrateOptions) -> DehydratedState {
    let states = client.query_states(QueryFilters::default());
    let queries = client
        .dehydrated()
        .into_iter()
        .filter_map(|(key, value, updated_at)| {
            let state = states.iter().find(|s| s.key == key)?;
            (options.should_dehydrate_query)(state).then_some(DehydratedQuery {
                key,
                value,
                updated_at,
            })
        })
        .collect();
    let mutations = client
        .mutation_states(&MutationFilters::default())
        .into_iter()
        .filter(|m| (options.should_dehydrate_mutation)(m))
        .map(|m| DehydratedMutation {
            mutation_key: m.mutation_key,
            submitted_at: m.submitted_at,
        })
        .collect();
    DehydratedState { queries, mutations }
}

/// Snapshots every cached query that was made with `persist`.
pub fn dehydrate(client: &QueryClient) -> DehydratedState {
    dehydrate_with(client, &DehydrateOptions::default())
}

/// Parks each saved value. A query made later with the same key and `persist`
/// starts from it, with its saved update time.
pub fn hydrate_with(client: &QueryClient, state: DehydratedState, options: &HydrateOptions) {
    for query in state.queries {
        if let Some(defaults) = &options.query_defaults {
            client.set_query_defaults(query.key.clone(), defaults.clone());
        }
        client.push_hydrated(query.key, query.value, query.updated_at);
    }
}

/// [`hydrate_with`] with no options.
pub fn hydrate(client: &QueryClient, state: DehydratedState) {
    hydrate_with(client, state, &HydrateOptions::default());
}

/// Saves [`dehydrate`]'s snapshot as JSON under `storage_key`, with `options`' buster and
/// the time of saving.
pub fn persist_to_storage_with(
    client: &QueryClient,
    storage_key: &str,
    options: &PersistOptions,
) -> Result<(), StorageFull> {
    let state = dehydrate(client);
    let queries: Vec<Value> = state
        .queries
        .iter()
        .map(|q| {
            json!({
                "key": serde_json::to_value(&q.key).unwrap_or(Value::Null),
                "value": q.value,
                "updated_at": q.updated_at,
            })
        })
        .collect();
    let mut root = Map::new();
    root.insert("queries".to_owned(), Value::Array(queries));
    root.insert("buster".to_owned(), Value::String(options.buster.clone()));
    root.insert("timestamp".to_owned(), json!(crate::now_ms()));
    Storage::set_item(storage_key, Value::Object(root).to_string())
}

/// [`persist_to_storage_with`] with the default options.
pub fn persist_to_storage(client: &QueryClient, storage_key: &str) -> Result<(), StorageFull> {
    persist_to_storage_with(client, storage_key, &PersistOptions::default())
}

/// Hydrates from what [`persist_to_storage_with`] saved. False when nothing valid is stored,
/// when the buster differs, or when the saved cache is older than `max_age`.
pub fn restore_from_storage_with(
    client: &QueryClient,
    storage_key: &str,
    options: &PersistOptions,
    hydrate_options: &HydrateOptions,
) -> bool {
    let Some(text) = Storage::get_item(storage_key) else {
        return false;
    };
    let Ok(Value::Object(root)) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    let Some(Value::Array(items)) = root.get("queries") else {
        return false;
    };
    let saved_buster = root.get("buster").and_then(Value::as_str).unwrap_or("");
    if saved_buster != options.buster {
        return false;
    }
    let saved_at = root.get("timestamp").and_then(Value::as_u64).unwrap_or(0);
    let age = crate::now_ms().saturating_sub(saved_at);
    if age > options.max_age.as_millis() as u64 {
        return false;
    }
    let mut queries = Vec::new();
    for item in items {
        let Some(key) = item
            .get("key")
            .and_then(|k| serde_json::from_value::<QueryKey>(k.clone()).ok())
        else {
            continue;
        };
        let value = item.get("value").cloned().unwrap_or(Value::Null);
        let updated_at = item.get("updated_at").and_then(Value::as_u64).unwrap_or(0);
        queries.push(DehydratedQuery {
            key,
            value,
            updated_at,
        });
    }
    hydrate_with(
        client,
        DehydratedState {
            queries,
            mutations: Vec::new(),
        },
        hydrate_options,
    );
    true
}

/// [`restore_from_storage_with`] with the default options.
pub fn restore_from_storage(client: &QueryClient, storage_key: &str) -> bool {
    restore_from_storage_with(
        client,
        storage_key,
        &PersistOptions::default(),
        &HydrateOptions::default(),
    )
}

/// TanStack's `useIsRestoring`. Restores in NSH run synchronously, so the flag is false in
/// every render: there is no restoring window to observe.
pub fn use_is_restoring() -> Memo<bool> {
    let client = query_client();
    computed(move || client.restoring().get())
}
