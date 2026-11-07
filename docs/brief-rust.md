# Brief: nsh-query Rust to TanStack Query parity

Read `docs/brief-common.md` in this worktree first.

## Goal

Bring the Rust crate `crates/nsh-query` to feature parity with TanStack Query v5, in snake_case Rust, on top of `gpui-query`'s GPUI-free `core`. Someone who knows TanStack Query must find the same concepts under the same names, with the same behavior. `@tanstack/query-core` cannot run in a Rust app, so this is a port of behavior: read `~/fo/.refs/tanstack-query/packages/query-core/src` (`query.ts`, `queryObserver.ts`, `queryClient.ts`, `mutation.ts`, `mutationObserver.ts`, `infiniteQueryBehavior.ts`, `retryer.ts`, `utils.ts`) and match what it does.

## Your paths

Worktree: `~/fo/.worktrees/nsh-query-tanstack`. You may edit only `crates/nsh-query/**` and `docs/parity-rust.md`.

Set this for every cargo command, because a build cache is already prepared there:

```bash
export CARGO_TARGET_DIR=/private/tmp/claude-501/-Users-hmziq--claude-spares-spare-NGupus/5378586f-1e26-4b17-89e9-b3eb305cfd38/scratchpad/verify/target-query
```

## Do not break existing callers

Other projects already use this crate. Every public name on `main` stays and keeps working: `use_query`, `use_query_with`, `use_query_by`, `use_mutation`, `use_mutation_with`, `use_infinite_query`, `query_client`, `QueryClient::{invalidate, cancel, remove, data, data_of, set_data, set_data_of, prefetch}`, `QueryOptions` and its builders, `MutationOptions` and its builders, `InfiniteQueryOptions`, and all `Query`, `Mutation` and `InfiniteQuery` methods. Add to them. All 16 existing tests must still pass unchanged.

Keep reusing `gpui_query`: `QueryResource`, `MutationResource`, `InfiniteQueryResource`, `CachePolicy`, `RetryPolicy`, `QueryKey`, `QueryKeyFilter`, `NetworkMode`, `RequestPolicy`, `QuerySignal`. Do not copy their logic. Add what the core lacks in this crate.

## Required additions

### Status

Add TanStack's two axes, derived from the core's `QueryStatus` and the active request:

- `Status::{Pending, Error, Success}` and `FetchStatus::{Fetching, Paused, Idle}`.
- `Query::status_v5()` and `Query::fetch_status()`. `Query::status()` keeps returning the core's value.

### Query result fields

Add each as a tracked method on `Query<T, E>`: `data_updated_at`, `error_updated_at`, `failure_count`, `failure_reason`, `error_update_count`, `is_error`, `is_fetched`, `is_fetched_after_mount`, `is_loading`, `is_loading_error`, `is_initial_loading`, `is_paused`, `is_placeholder_data`, `is_refetch_error`, `is_refetching`, `is_stale`, `is_enabled`. `is_pending`, `is_fetching`, `is_success` and `is_failure` exist already. Note TanStack's meaning: `is_loading` is `is_fetching && is_pending`.

### Query options

Add builders on `QueryOptions`: `stale_time(Duration)` as a synonym for a TTL cache policy, `gc_time(Duration)` as a synonym for `gc`, `retry_if(Fn(u32, &E) -> bool)`, `retry_delay(Fn(u32, &E) -> Duration)`, `network_mode(NetworkMode)`, `refetch_interval_fn(Fn() -> Option<Duration>)`, `refetch_interval_in_background(bool)`, `refetch_on_mount(RefetchTrigger)`, `refetch_on_window_focus(RefetchTrigger)` as a synonym for `refetch_on_foreground`, `refetch_on_reconnect(RefetchTrigger)`, `retry_on_mount(bool)`, `initial_data(Fn() -> T)`, `initial_data_updated_at(u64)`, `placeholder_data(Fn(Option<&T>) -> Option<T>)`, `keep_previous_data()`, `meta(serde_json::Value)`.

Options that carry `T` or `E` may need a typed `QueryConfig<T, E>` alongside the untyped `QueryOptions`. Add `use_query_config(key, QueryConfig<T, E>, fetcher)` if so, and keep the existing functions. `select` already exists as `Query::select`. `notifyOnChangeProps` and `structuralSharing` are `not applicable`, since signals are fine-grained, and the parity file should say that.

### QueryClient

Add `QueryFilters { query_key, exact, query_type, stale, fetch_status, predicate }`, and `QueryType::{All, Active, Inactive}`, with `From<&str>` and `From<QueryKey>` so existing calls compile. Add: `fetch_query`, `ensure_query_data`, `get_query_data`, `get_queries_data`, `set_query_data`, `update_query_data` (TanStack's updater form), `set_queries_data`, `get_query_state`, `invalidate_queries`, `refetch_queries`, `cancel_queries`, `remove_queries`, `reset_queries`, `is_fetching`, `is_mutating`, `set_default_options`, `get_default_options`, `set_query_defaults`, `get_query_defaults`, `set_mutation_defaults`, `get_mutation_defaults`, `clear`, `resume_paused_mutations`, `fetch_infinite_query`, `prefetch_infinite_query`, `ensure_infinite_query_data`.

### Managers

Add `focus_manager()` over NSH's `lifecycle()` and `online_manager()` with `set_online(bool)` and `is_online()`. `NetworkMode::Online` pauses fetches while offline, with `FetchStatus::Paused`, and resumes them when back online. Mutations pause the same way. Reconnect honors `refetch_on_reconnect`.

### Mutations

Add a context type: `MutationOptions<V, T, E, C = ()>`, where `on_mutate` may return `C`. Add `on_success_ctx`, `on_error_ctx` and `on_settled_ctx`, which also receive the context. Existing builders still compile. Add `mutation_key`, `scope(id)` running calls of one scope in series, `retry_delay`, `network_mode`, `gc_time`. Add methods: `is_idle`, `is_pending`, `is_paused`, `failure_count`, `failure_reason`, `submitted_at`, `context`. Add a mutation cache on the client, so `is_mutating`, `use_is_mutating` and `use_mutation_state` work.

### Hooks

`use_queries(Vec<(key, options)>, fetcher)` with a `combine` variant, `use_is_fetching(filters)`, `use_is_mutating(filters)`, `use_mutation_state(filters, select)`.

### Infinite queries

Move infinite entries into the client cache, so two observers of one key share pages and they survive a remount inside `gc_time`. Add `page_params()`, `is_fetch_next_page_error()`, `is_fetch_previous_page_error()`, `data_updated_at()`. Refetching refetches every loaded page in order, as `infiniteQueryBehavior.ts` does. Add a `reset()` for what `refetch()` does today, then change `refetch()` to the TanStack meaning, and say so in the parity file, as the one deliberate behavior change.

### Persistence

Behind a `persist` feature with `serde_json`: `dehydrate(&client)`, `hydrate(&client, state)` and `persist_to_storage`, using NSH's storage from `nsh_core`. Find it in `~/.cargo/registry/src/*/nsh-core-0.4.0/src/storage.rs`. Only entries registered with a serializer persist.

## Required tests

Add these to `crates/nsh-query/tests/`, using `Headless` on a manual clock. Each must exist by this name and pass:

`status_and_fetch_status_follow_a_load`, `is_loading_is_fetching_and_pending`, `failure_count_and_reason_track_retries`, `retry_if_stops_early`, `retry_delay_function_is_honored`, `stale_time_and_gc_time_synonyms_work`, `initial_data_counts_as_fresh_with_updated_at`, `placeholder_data_and_keep_previous_data_flag_placeholder`, `refetch_on_mount_trigger_is_honored`, `refetch_interval_stops_in_background`, `refetch_on_foreground_when_stale`, `offline_pauses_and_online_resumes`, `reconnect_refetches_when_asked`, `query_filters_exact_type_stale_predicate`, `fetch_query_and_ensure_query_data`, `update_query_data_uses_the_updater`, `set_queries_data_and_get_queries_data`, `reset_queries_goes_back_to_initial`, `query_defaults_apply_by_key_prefix`, `mutation_context_reaches_error_and_settled`, `mutation_scope_runs_in_series`, `mutation_pauses_offline_and_resumes`, `use_is_fetching_and_is_mutating_count`, `use_mutation_state_lists_calls`, `use_queries_with_combine`, `infinite_refetch_reloads_all_pages`, `infinite_shared_between_observers`, `infinite_survives_remount_within_gc`, `dehydrate_and_hydrate_round_trip`.

## Docs

- `docs/parity-rust.md`: a table with one row for every name in `docs/brief-typescript.md`'s query result, `QueryClient`, option and mutation lists, giving the Rust name. Columns: TanStack name, Rust name, status (`done`, `not applicable`, or `not done`), the test that covers it, a note.
- Update `crates/nsh-query` doc comments. Do not edit the repository README; the agent that started you will.

## Finish conditions

You are finished only when all of these hold. Run them yourself and paste the output in your report.

1. `cargo fmt --all -- --check` exits 0.
2. `cargo clippy -p nsh-query --all-targets --all-features -- -D warnings` exits 0.
3. `cargo test -p nsh-query --all-features` exits 0, with the original 16 tests unchanged and every test named above present and passing.
4. `git diff`-free check of compatibility: a small `tests/compat.rs` uses every public name listed under "Do not break existing callers", and compiles.
5. `docs/parity-rust.md` has every required row, and every row marked `done` names a test that exists.
6. You broke the code once on purpose for each of `offline_pauses_and_online_resumes`, `mutation_scope_runs_in_series` and `infinite_refetch_reloads_all_pages`, and saw each fail. Say how in your report.
7. No `std::thread::sleep`, no `std::time::Instant` and no `SystemTime::now` in `crates/nsh-query/src`. Time comes from `nsh_core::time`.
