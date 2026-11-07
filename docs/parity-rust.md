# nsh-query (Rust) parity with TanStack Query v5

Reference: `@tanstack/query-core` source in `~/fo/.refs/tanstack-query`. Each row is one TanStack name, its Rust name, its status, the test that covers it, and a note.

Status values: `done` means the behavior is implemented and a named test checks it. `not applicable` means Rust has no such concept, and its note names what replaces it.

## Table

| TanStack name | Rust name | Status | Test | Note |
|---|---|---|---|---|
| `data` | `data()` | done | `two_observers_of_one_key_share_one_load` | Needs `T: Clone`, as before. |
| `dataUpdatedAt` | `data_updated_at()` | done | `result_fields_report_the_load_state` | Milliseconds since the Unix epoch, 0 before any data. |
| `error` | `error()` | done | `a_failure_with_no_retries_left_reports_the_error` | Needs `E: Clone`, as before. |
| `errorUpdatedAt` | `error_updated_at()` | done | `result_fields_report_the_load_state` | Set when a load gives up, not on each retry. |
| `failureCount` | `failure_count()` | done | `failure_count_and_reason_track_retries` | Counts failed attempts in the current load; reset on start and on success. |
| `failureReason` | `failure_reason()` | done | `failure_count_and_reason_track_retries` | During retries, the latest attempt's error. After a final failure, the stored error. Needs `E: Clone`. |
| `errorUpdateCount` | `error_update_count()` | done | `result_fields_report_the_load_state` | Counts loads that ended in an error. |
| `isError` | `is_error()` | done | `result_fields_report_the_load_state` | TanStack's `status === 'error'`, from `status_v5()`. |
| `isFetched` | `is_fetched()` | done | `result_fields_report_the_load_state` | True once a load has settled, with data or an error. |
| `isFetchedAfterMount` | `is_fetched_after_mount()` | done | `is_initial_loading_and_fetched_after_mount_follow_tanstack` | Matches TanStack when there is no initial data, because its counts start at zero. |
| `isFetching` | `is_fetching()` | done | `is_loading_is_fetching_and_pending` | True while a load runs and is not paused. |
| `isLoading` | `is_loading()` | done | `is_loading_is_fetching_and_pending` | `is_fetching() && status_v5() == Pending`. |
| `isPending` | `is_pending()` | done | `is_success_and_is_pending_follow_tanstack_status` | TanStack status pending. Changed from the core meaning. Same value as status_v5() == Pending. |
| `isLoadingError` | `is_loading_error()` | done | `result_fields_report_the_load_state` | Error status with no data. |
| `isInitialLoading` | `is_initial_loading()` | done | `is_initial_loading_and_fetched_after_mount_follow_tanstack` | Same value as is_loading(). |
| `isPaused` | `is_paused()` | done | `offline_pauses_and_online_resumes` | `fetch_status() == Paused`. |
| `isPlaceholderData` | `is_placeholder_data()` | done | `placeholder_data_and_keep_previous_data_flag_placeholder` | True while placeholder data or kept previous data is shown. |
| `isRefetchError` | `is_refetch_error()` | done | `result_fields_report_the_load_state` | Error status with data still present. |
| `isRefetching` | `is_refetching()` | done | `is_loading_is_fetching_and_pending` | A load runs while data or an error is already shown. |
| `isStale` | `is_stale()` | done | `result_fields_report_the_load_state` | Stale when there is no data, the data is past `stale_time`, or it was invalidated. Disabled queries are not stale. |
| `isSuccess` | `is_success()` | done | `is_success_and_is_pending_follow_tanstack_status` | TanStack status success, so it stays true during a refetch. Changed from the core meaning. |
| `isEnabled` | `is_enabled()` | done | `result_fields_report_the_load_state` | Tracked: reads the `enabled` condition. |
| `refetch` | `refetch()` | done | `offline_pauses_and_online_resumes` | Forced load, ignoring freshness. |
| `status` | `status_v5() (TanStack) and status() (core)` | done | `status_and_fetch_status_follow_a_load` | `status()` still returns gpui-query's `QueryStatus`, as the brief requires. |
| `fetchStatus` | `fetch_status()` | done | `status_and_fetch_status_follow_a_load` | `Fetching`, `Paused` or `Idle`. |
| `fetchNextPage` | `fetch_next_page()` | done | `infinite_refetch_reloads_all_pages` | No-op past the last page or while a next page loads. |
| `fetchPreviousPage` | `fetch_previous_page()` | done | `pages_load_in_both_directions_and_stop_at_the_ends` | Loads before the first page, and stops at the ends. |
| `hasNextPage` | `has_next_page()` | done | `pages_load_in_both_directions_and_stop_at_the_ends` | Checked by the same test as fetchPreviousPage. |
| `hasPreviousPage` | `has_previous_page()` | done | `pages_load_in_both_directions_and_stop_at_the_ends` | Checked by the same test as fetchPreviousPage. |
| `isFetchNextPageError` | `is_fetch_next_page_error()` | done | `fetch_next_page_error_flag_is_set_for_next_only` | Set only by a failed next-page load. |
| `isFetchingNextPage` | `is_fetching_next_page()` | done | `infinite_fetching_flags_and_page_params_follow_tanstack` |  |
| `isFetchPreviousPageError` | `is_fetch_previous_page_error()` | done | `fetch_previous_page_error_flag_is_set_for_previous_only` | Set only by a failed previous-page load. |
| `isFetchingPreviousPage` | `is_fetching_previous_page()` | done | `infinite_fetching_flags_and_page_params_follow_tanstack` |  |
| `pageParams` | `page_params()` | done | `infinite_fetching_flags_and_page_params_follow_tanstack` | First page first. |
| `data` | `data()` | done | `mutation_data_is_success_and_is_pending_follow_the_call` |  |
| `variables` | `variables()` | done | `mutation_fields_report_the_call` | Variables of the latest call. |
| `error` | `error()` | done | `an_optimistic_write_rolls_back_after_the_last_attempt_fails` | Needs `E: Clone`. |
| `isError` | `is_failure()` | done | `mutation_fields_report_the_call` | Named `is_failure` in Rust, as before. |
| `isIdle` | `is_idle()` | done | `mutation_fields_report_the_call` | True before the first call and after `reset`. |
| `isPending` | `is_pending()` | done | `mutation_data_is_success_and_is_pending_follow_the_call` | Loading state. Idle and success are checked too. |
| `isSuccess` | `is_success()` | done | `mutation_data_is_success_and_is_pending_follow_the_call` |  |
| `status` | `status()` | done | `mutation_pauses_offline_and_resumes` | Core `MutationStatus`: Idle, Loading, Success, Failure. |
| `mutate` | `mutate()` | done | `mutation_pauses_offline_and_resumes` | Superseded calls drop their result and callbacks. |
| `mutateAsync` | `mutate_async()` | done | `mutate_async_replaced_call_resolves_with_its_own_result` | A replaced call resolves with its own result. The Option is kept for existing callers and is always Some. |
| `reset` | `reset()` | done | `mutation_reset_returns_to_idle` | Clears status, data, variables and context. |
| `context` | `context()` | done | `mutation_fields_report_the_call` | Needs `C: Clone`. Returns what `on_mutate` gave for the latest call. |
| `failureCount` | `failure_count()` | done | `mutation_fields_report_the_call` | Failed attempts in the current call. |
| `failureReason` | `failure_reason()` | done | `mutation_fields_report_the_call` | Needs `E: Clone`. |
| `isPaused` | `is_paused()` | done | `mutation_pauses_offline_and_resumes` | True while waiting for the network. |
| `submittedAt` | `submitted_at()` | done | `mutation_fields_report_the_call` | Milliseconds since the Unix epoch. |
| `cancelQueries` | `cancel_queries(filters)` | done | `cancel_queries_keeps_data_and_drops_the_late_answer` | Keeps data, stops the fetch, ignores the late answer. |
| `clear` | `clear()` | done | `dehydrate_and_hydrate_round_trip` | Removes queries and mutations. Observed queries stay until their observers go. |
| `ensureInfiniteQueryData` | `ensure_infinite_query_data()` | done | `ensure_infinite_query_data_returns_cache_then_loads` |  |
| `ensureQueryData` | `ensure_query_data()` | done | `fetch_query_and_ensure_query_data` | Returns cached data as it is, else loads it. |
| `fetchInfiniteQuery` | `fetch_infinite_query()` | done | `fetch_infinite_query_always_reloads_the_pages` | Reloads every time, as TanStack's fetchInfiniteQuery does with staleTime 0. |
| `fetchQuery` | `fetch_query()` | done | `fetch_query_and_ensure_query_data` | Returns `Result<T, FetchQueryError<E>>`. A cancelled load returns `FetchQueryError::Cancelled`, where TanStack throws. |
| `getDefaultOptions` | `get_default_options()` | done | `get_default_options_and_set_default_options_apply_to_new_queries` |  |
| `getMutationCache` | `get_mutation_cache()` | done | `mutation_cache_subscribe_stops_and_remove_drops_settled_calls` | find, find_all, get_all, subscribe (returns a stop) and remove. Its clear removes settled calls, as TanStack's does. |
| `getMutationDefaults` | `get_mutation_defaults()` | done | `get_mutation_defaults_and_set_mutation_defaults_apply_retry` |  |
| `getQueriesData` | `get_queries_data()` | done | `set_queries_data_and_get_queries_data` | Typed by `T` and `E`: only queries of that type are returned. |
| `getQueryCache` | `-` | done | `query_cache_finds_lists_subscribes_and_removes` | `get_query_cache()`: find, find_all, get_all, subscribe (with a stop), clear and remove. |
| `getQueryData` | `get_query_data()` | done | `get_query_data_and_get_query_state_read_the_cache` |  |
| `getQueryDefaults` | `get_query_defaults()` | done | `get_query_defaults_reads_back_a_prefix_default` |  |
| `getQueryState` | `get_query_state()` | done | `get_query_data_and_get_query_state_read_the_cache` | Reports status and update counts. |
| `infiniteQuery` | `fetch_infinite_query() (closest)` | done | `client_infinite_query_reloads_like_fetch_infinite_query` | `QueryClient::infinite_query` is TanStack's `client.infiniteQuery`, the awaitable fetch. It reloads, as TanStack's default staleTime of zero does. Infinite entries have no stale time of their own. |
| `invalidateQueries` | `invalidate_queries(filters)` | done | `invalidating_replaces_a_load_in_flight_and_drops_its_late_answer` | That test calls `invalidate(prefix)`, which delegates to `invalidate_queries`. |
| `isFetching` | `is_fetching(filters)` | done | `use_is_fetching_and_is_mutating_count` | Exercised through `use_is_fetching`. |
| `isMutating` | `is_mutating(filters)` | done | `use_is_fetching_and_is_mutating_count` | Exercised through `use_is_mutating`. |
| `mount` | `-` | not applicable | - | NSH's managers are set up by the surface. No client call mounts them. |
| `prefetchInfiniteQuery` | `prefetch_infinite_query()` | done | `prefetch_infinite_query_loads_once_for_a_later_observer` |  |
| `prefetchQuery` | `prefetch() (kept name)` | done | `query_filters_exact_type_stale_predicate` | Kept under its existing Rust name. |
| `refetchQueries` | `refetch_queries(filters)` | done | `query_filters_exact_type_stale_predicate` | Refetches every match, active or not. |
| `removeQueries` | `remove_queries(filters)` | done | `dehydrate_and_hydrate_round_trip` | Exercised through `clear()`. An observed query is dropped when its observer goes. |
| `resetQueries` | `reset_queries(filters)` | done | `reset_queries_goes_back_to_initial` | Back to the initial state, then reloads the observed ones. |
| `resumePausedMutations` | `resume_paused_mutations()` | done | `resume_paused_mutations_runs_mutations_but_not_queries` | Continues paused mutations even offline, as TanStack does. Paused queries stay paused. |
| `setDefaultOptions` | `set_default_options()` | done | `get_default_options_and_set_default_options_apply_to_new_queries` |  |
| `setMutationDefaults` | `set_mutation_defaults()` | done | `get_mutation_defaults_and_set_mutation_defaults_apply_retry` |  |
| `setQueriesData` | `set_queries_data(filters, updater)` | done | `set_queries_data_and_get_queries_data` | Updater receives the current value and returns the new one. |
| `setQueryData` | `set_query_data() / set_data()` | done | `update_query_data_uses_the_updater` | Seeded through `set_data`. |
| `setQueryDefaults` | `set_query_defaults(prefix, options)` | done | `query_defaults_apply_by_key_prefix` | The longest matching prefix wins, over the client default. |
| `unmount` | `-` | not applicable | - | See `mount`. |
| `staleTime` | `QueryOptions::stale_time()` | done | `stale_time_and_gc_time_synonyms_work` | Synonym of `ttl`. |
| `gcTime` | `QueryOptions::gc_time()` | done | `stale_time_and_gc_time_synonyms_work` | Synonym of `gc`. |
| `retry (function)` | `QueryConfig::retry_if()` | done | `retry_if_stops_early` | Typed on `QueryConfig`, because it takes `E`. Replaces the retry policy. |
| `retryDelay` | `QueryConfig::retry_delay()` | done | `retry_delay_function_is_honored` | Typed on `QueryConfig`, because it takes `E`. |
| `networkMode` | `QueryOptions::network_mode()` | done | `network_mode_always_runs_while_offline` | Always and OfflineFirst are covered (network_mode_offline_first_runs_the_first_attempt_then_pauses_retries). Online is the default in offline_pauses_and_online_resumes. |
| `refetchInterval (function)` | `QueryOptions::refetch_interval_fn()` | done | `refetch_interval_fn_stops_when_it_returns_none` | Asked again after each fetch settles, as TanStack does. |
| `refetchIntervalInBackground` | `QueryOptions::refetch_interval_in_background()` | done | `refetch_interval_in_background_keeps_ticking` |  |
| `refetchOnMount` | `QueryOptions::refetch_on_mount()` | done | `refetch_on_mount_trigger_is_honored` | `Always`, `IfStale` and `Never` map to TanStack's `'always'`, `true` and `false`. |
| `refetchOnWindowFocus` | `QueryOptions::refetch_on_window_focus()` | done | `refetch_on_window_focus_alias_sets_the_foreground_trigger` | Alias of refetch_on_foreground. |
| `refetchOnReconnect` | `QueryOptions::refetch_on_reconnect()` | done | `reconnect_refetches_when_asked` | `Always` refetches on reconnect. `IfStale` only when stale. |
| `retryOnMount` | `QueryOptions::retry_on_mount()` | done | `retry_on_mount_false_skips_a_mount_after_an_error` | Only skips when there is no data, as TanStack's shouldLoadOnMount does. |
| `initialData` | `QueryConfig::initial_data()` | done | `initial_data_counts_as_fresh_with_updated_at` | Only used when nothing is cached. Typed on `QueryConfig`. |
| `initialDataUpdatedAt` | `QueryOptions::initial_data_updated_at()` | done | `initial_data_counts_as_fresh_with_updated_at` | Decides whether the seed is fresh or refetches on mount. |
| `placeholderData` | `QueryConfig::placeholder_data()` | done | `placeholder_data_and_keep_previous_data_flag_placeholder` | Typed on `QueryConfig`. It gets the previous key's data. |
| `keepPreviousData` | `QueryOptions::keep_previous_data()` | done | `placeholder_data_and_keep_previous_data_flag_placeholder` | Shows the previous key's data as placeholder data while the new key loads. |
| `meta` | `QueryOptions::meta()` | done | `meta_reaches_the_fetcher_context` | Passed to the fetcher as QueryContext::meta. Infinite fetchers receive None. |
| `enabled` | `QueryOptions::enabled()` | done | `a_disabled_query_waits_for_its_condition` | Tracked: a signal read in the closure re-runs the query when it changes. |
| `skipToken` | ``enabled` set to false` | done | `skip_token_maps_to_enabled_false` | Maps to `enabled(false)`. |
| `select` | `Query::select()` | done | `select_maps_the_data` |  |
| `notifyOnChangeProps` | `-` | done | `a_binding_on_one_query_field_wakes_only_for_that_field` | Each read subscribes to its own memo, so a binding wakes only for the field it reads. Proved by tests/fine_grained.rs. |
| `structuralSharing` | `-` | done | `structural_sharing_keeps_a_binding_quiet_for_an_equal_answer` | `QueryConfig::structural_sharing(equal)`. An equal answer does not bump the data version, so a `data()` binding does not run again. |
| `mutationKey` | `MutationOptions::mutation_key()` | done | `use_is_fetching_and_is_mutating_count` | Used by `use_is_mutating` to match calls. |
| `scope` | `MutationOptions::scope()` | done | `mutation_scope_runs_in_series` | Calls in one scope run one at a time, in start order. |
| `retryDelay (mutation)` | `MutationOptions::retry_delay()` | done | `mutation_retry_delay_gets_the_failures_before_this_one` | Receives the failures before the current one, as TanStack's retryer does. |
| `networkMode (mutation)` | `MutationOptions::network_mode()` | done | `mutation_network_mode_always_runs_while_offline` | OfflineFirst is covered by mutation_network_mode_offline_first_runs_once_then_waits_for_retries. |
| `gcTime (mutation)` | `MutationOptions::gc_time()` | done | `mutation_gc_time_keeps_a_settled_call_listed_then_removes_it` | A settled call with no observer leaves the cache after gc_time. The timer runs under a detached owner, so it outlives the component. |
| `onMutate` | `MutationOptions::on_mutate()` | done | `mutation_context_reaches_error_and_settled` | Changes the context type `C`. Call it before any `*_ctx` hook. |
| `onSuccess` | `MutationOptions::on_success()` | done | `a_success_invalidates_and_the_query_loads_the_new_value` |  |
| `onSuccess (context)` | `MutationOptions::on_success_ctx()` | done | `mutation_on_success_ctx_receives_the_context` |  |
| `onError` | `MutationOptions::on_error()` | done | `an_optimistic_write_rolls_back_after_the_last_attempt_fails` |  |
| `onError (context)` | `MutationOptions::on_error_ctx()` | done | `mutation_context_reaches_error_and_settled` |  |
| `onSettled` | `MutationOptions::on_settled()` | done | `a_mutation_in_flight_finishes_after_its_component_goes` |  |
| `onSettled (context)` | `MutationOptions::on_settled_ctx()` | done | `mutation_context_reaches_error_and_settled` |  |
| `useQueries` | `use_queries()` | done | `use_queries_with_combine` | Exercised through `use_queries_combined`, which calls it. |
| `useQueries with combine` | `use_queries_combined()` | done | `use_queries_with_combine` | Returns a memo of the combined value. |
| `useIsFetching` | `use_is_fetching()` | done | `use_is_fetching_and_is_mutating_count` |  |
| `useIsMutating` | `use_is_mutating()` | done | `use_is_fetching_and_is_mutating_count` |  |
| `useMutationState` | `use_mutation_state()` | done | `use_mutation_state_lists_calls` | Returns a memo of the mapped states, oldest first. |
| `focusManager` | `focus_manager().is_focused()` | done | `refetch_interval_keeps_running_while_inactive` | Focused means `lifecycle() != Background`, matching TanStack's visibility check. |
| `onlineManager` | `online_manager().set_online() / is_online()` | done | `offline_pauses_and_online_resumes` | `is_online()` is read by the query code, not called by a test. |
| `dehydrate` | `dehydrate()` | done | `dehydrate_and_hydrate_round_trip` | Persist feature. Only queries made with `persist()` are included. |
| `hydrate` | `hydrate()` | done | `dehydrate_and_hydrate_round_trip` | Persist feature. Values wait for a query with the same key. |
| `persistQueryClient / createStoragePersister` | `persist_to_storage() / restore_from_storage()` | done | `dehydrate_and_hydrate_round_trip` | Persist feature. JSON in NSH storage under one key. |
| `persist option (serializer)` | `QueryConfig::persist()` | done | `dehydrate_and_hydrate_round_trip` | Persist feature. Only registered queries persist. |
| `mutate with options` | `mutate_with(variables, MutateOptions)` | done | `mutate_with_callbacks_run_for_the_latest_call_only` | Per-call callbacks run only for the latest call, and only while the component is alive. |
| `usePrefetchQuery` | `use_prefetch_query()` | done | `use_prefetch_query_loads_for_a_later_observer` | Loads into the cache when the component is built. It does not observe. |
| `usePrefetchInfiniteQuery` | `use_prefetch_infinite_query()` | done | `use_prefetch_infinite_query_serves_a_later_observer` | Loads the first page when the component is built. |
| `throwOnError` | `QueryConfig::throw_on_error() / MutationOptions::throw_on_error()` | done | `throw_on_error_reaches_catch_error_and_the_reset_boundary_recovers` | On `QueryConfig` and `MutationOptions`. Reading a failed value panics, so the nearest `catch_error` shows it (mutation: mutation_throw_on_error_reaches_catch_error). |
| `useQueryErrorResetBoundary` | `use_query_error_reset_boundary()` | done | `throw_on_error_reaches_catch_error_and_the_reset_boundary_recovers` | `use_query_error_reset_boundary().reset()` reloads the failed queries. |
| `matchQuery` | `match_query()` | done | `match_query_and_hash_key_follow_tanstack` | Matches a query's state against the filters, as TanStack does. |
| `matchMutation` | `match_mutation()` | done | `match_mutation_matches_the_cached_state` | Matches a mutation's state against filters. |
| `hashKey` | `hash_key()` | done | `match_query_and_hash_key_follow_tanstack` | A stable JSON array of the key's parts. Keys compare by value, so the hash is for logs and other stores. |
| `subscribed` | `QueryOptions::subscribed()` | done | `subscribed_false_reads_the_cache_and_never_fetches`, `unsubscribed_entry_leaves_the_cache_while_its_component_lives` | Reads the cache under the current key, and never fetches or observes. Its entry leaves the cache after gc_time while the component is still on screen, as TanStack's does. The entry's data stays readable by that component. |
| `queryKeyHashFn` | `-` | not applicable | - | Rust keys compare by value (`QueryKey: Eq`), so the cache has no hash to replace. Owner to review. |
| `QueryContext client` | `QueryContext::client` | done | `fetcher_context_carries_meta_client_and_direction` | `QueryContext::client`. |
| `InfiniteQuery context page_param and direction` | `QueryContext::direction (page param is the first argument)` | done | `fetcher_context_carries_meta_client_and_direction` | `QueryContext::direction` is set for infinite pages; the page param is the fetcher's first argument. |
| `InfiniteQueryOptions meta` | `InfiniteQueryOptions::meta()` | done | `fetcher_context_carries_meta_client_and_direction` | Passed to every page fetch as `QueryContext::meta`. |
| `dehydrate should_dehydrate_query` | `DehydrateOptions` | done | `persistence_options_select_restore_and_expire` | `DehydrateOptions::should_dehydrate_query`. |
| `dehydrate should_dehydrate_mutation` | `DehydrateOptions` | done | `persistence_options_select_restore_and_expire` | Lists matching mutations in the saved state as information. A restore does not start them again, since a mutation belongs to its component. |
| `hydrate options` | `HydrateOptions` | done | `persistence_options_select_restore_and_expire` | `HydrateOptions::query_defaults`. |
| `persist maxAge` | `persist options` | done | `persistence_options_select_restore_and_expire` | `PersistOptions::max_age`: a saved cache older than this is not restored. |
| `persist buster` | `persist options` | done | `persistence_options_select_restore_and_expire` | `PersistOptions::buster`: a different buster discards the saved cache. |
| `useIsRestoring` | `use_is_restoring()` | done | `persistence_options_select_restore_and_expire` | `use_is_restoring()`. Restores are synchronous here, so the flag is false in every render; there is no restoring window to see. |
| `useQueries per index` | `use_queries()` | done | `use_queries_results_are_fine_grained_per_index` | Each index is its own query, so a binding on one index does not wake for another. |
| `client.query` | `-` | done | `client_query_returns_fresh_cached_data_without_fetching` | `QueryClient::query`, with TanStack's staleTime (fresh data is served), retry off by default, enabled ignored, and `query_selected` for select. |

## Decisions

1. **Infinite `refetch()` changed to TanStack's meaning.** It reloads every loaded page in order, each from its own param. `reset()` keeps the old meaning: drop the pages and load from the first. The owner changed the test's refetch button to call `reset()`; the assertion `pages 2` is unchanged and passes.
2. **Default `stale_time` stays at 60 s.** The crate's default TTL predates this work. TanStack's default is 0. `stale_time(Duration::ZERO)` gives TanStack's behavior.
3. **Focus means visible.** `Active` and `Inactive` count as focused and `Background` does not, matching TanStack's `visibilityState !== "hidden"` and the crate's behavior on `main`.
4. **Status follows TanStack.** `status_v5()` and `fetch_status()` are TanStack's. `is_pending`, `is_success` and `is_failure` on `Query` now use `status_v5()`, so a refetch after data keeps `is_success` true. The mutation `is_success` and `is_failure` are unchanged.
5. **`fetch_query` and `ensure_query_data` return `Result<T, FetchQueryError<E>>`.** `Failed(E)` carries the error. `Cancelled` replaces TanStack's thrown `CancelledError`. A cancelled load with no data keeps the future pending until a later load settles.
6. **Typed options live on `QueryConfig<T, E>`.** `retry_if`, `retry_delay`, `initial_data`, `placeholder_data` and `persist` take `T` or `E`, and `QueryOptions` has neither. `QueryConfig` converts from `QueryOptions`, and `use_query_config` takes it. The untyped `QueryOptions` builders are unchanged.
7. **Mutation context.** `on_mutate` changes the context type `C`, so call it before any `*_ctx` hook: it drops hooks set earlier. `use_mutation_with` keeps its signature (`C = ()`). `use_mutation_with_context` is the `C`-typed entry point.
8. **Client defaults merge by equality.** A call's option that equals the built-in default is replaced by the client default for that key. A call cannot override a client default with the built-in value.
9. **Persist adds two optional dependencies.** `serde` and `serde_json` are enabled by the `persist` feature. The workspace `Cargo.lock` gained both under `nsh-query`. That file sits outside this crate's paths, so the change is flagged here.
10. **Mutation cache lifetime.** A mutation leaves the cache when its component goes and the call has settled, once `gc_time` has passed.
11. **Infinite entries use a fixed 300 s gc.** They are cached in the client, so they survive a remount within that time. No option sets it yet.
12. **Each query read follows its own memo.** `Query::data`, `fetch_status`, `is_stale` and the other reads subscribe to one memo over the entry each, so a binding wakes only for the field it reads. The data itself is versioned, since it has no `PartialEq`.
13. **A cancelled query keeps the core status.** `cancel()` ends the load and clears `fetch_status`, but the core status can stay `LoadingWithData`. `status_v5()` is unaffected.
14. **Core time source.** gpui-query's `MutationResource` stamps completions with `SystemTime::now`. That code is in the dependency, not in this crate's `src`, which uses only `nsh_core::time`.

- **Retry callbacks get the failures before this one.** `retry_if` and `retry_delay` receive 0 on the first failure, as TanStack's retryer passes its count before incrementing. Both query and mutation follow this.
- **`resume_paused_mutations` resumes mutations even offline.** Paused queries stay paused, as in TanStack.
- **`fetch_infinite_query` always reloads.** TanStack's `fetchInfiniteQuery` runs `fetchQuery` with a zero stale time.
- **The refetch interval function is asked after each fetch settles,** as TanStack asks it after each result, so it sees the new data.
- **Mount loads without data, unless a retry on mount is refused after an error.** With data, `refetch_on_mount` decides, as TanStack's `shouldLoadOnMount` and `shouldFetchOnMount` do.
- **`QueryContext::meta`** carries `QueryOptions::meta`. Infinite fetchers receive `None`, because infinite options have no meta.

- **Infinite queries are unbounded by default.** The core defaults to 50 pages. NSH now passes `max_pages` through as given, so `None` means no limit, as in TanStack. `max_pages(n)` still caps the list, and `unbounded_pages()` undoes it.
- **An unobserved entry starts its gc when it is made.** `set_data`, `prefetch`, `fetch_query` and infinite-query entries leave the cache after gc_time if nothing observes them. An observer or a handle that starts later cancels or restarts that timer. Tested by `an_unobserved_seeded_entry_leaves_after_its_gc_time` and `an_unobserved_infinite_entry_leaves_after_its_gc_time`.
- **Two counts keep an entry alive.** `observers` follow the entry and stop its gc. `holders` are handles that read it without following it, as `subscribed(false)` does. A holder keeps the entry's signals alive after the cache drops it, so its component can still read the last data, but the holder does not stop the gc.
- **`Fetched` carries only the cache policy.** A fetcher's `Fetched::cache_policy` replaces the entry's on success. Its `meta` is for persistence and is not read here.
- **Manual queries keep their fetcher.** `use_query_manual` and `use_query_manual_opts` load nothing on mount or on a key change. `refetch()` loads with the fetcher they were given.
- **Placeholder and previous data are read untracked.** Both change only with the key, and a key change always sets `current`, which every reader follows. The previous value is written before `current`, so a reader that re-runs on the change sees it.
- **`diagnostics()` is a snapshot.** `use_diagnostics()` is the memo a binding follows. The cache age in a snapshot is as of the last change to the cache.
- **`with_config` is not applicable.** It configured gpui-query's `Observer`, which notifies only when a status or the data changes. NSH observers are internal, and every binding already wakes only for what it reads (`tests/fine_grained.rs`).

## Open rows

No row is open. Each table row is `done` or `not applicable`, and each `done` row names the test that checks it.

## gpui-query parity

Every public item of `~/fo/.refs/gpui-query/crates/gpui-query/src/{hook,client}`, plus `gpui-query-http` and `gpui-query-persist`. Rows marked `not applicable` are core or internal items, and the note names what replaces them.

| gpui-query name | Rust name | Status | Test | Note |
|---|---|---|---|---|
| `PersistError` | `-` | not applicable | - | Persistence errors are `StorageFull` from NSH storage here. |
| `PersistFilter` | `should_dehydrate_query / should_dehydrate_mutation` | done | `dehydrate_and_hydrate_round_trip` | Filters are `DehydrateOptions` closures. |
| `all_infinite_queries` | `QueryClient::all_infinite_queries()` | done | `all_infinite_queries_lists_only_the_page_type_asked_for` | Snapshots of the infinite queries of one page, value and error type. It returns `QueryState`, not handles. |
| `all_mutations` | `get_mutation_cache().get_all()` | done | `mutation_cache_callbacks_fire_for_each_point_of_a_call` | Lists cached mutations through the mutation cache. |
| `all_queries` | `get_query_cache().get_all()` | done | `query_cache_finds_lists_subscribes_and_removes` | Lists cached queries through the query cache. |
| `cache_policy` | `QueryOptions::cache()` | done | `a_fresh_entry_skips_the_load_and_an_expired_one_reloads` | Cache policy is an option; the existing cache tests exercise it. |
| `cancel_queries` | `QueryClient::cancel_queries()` | done | `cancel_queries_keeps_data_and_drops_the_late_answer` | Stops loads and keeps data. |
| `collect_persist_snapshot` | `dehydrate()` | done | `dehydrate_and_hydrate_round_trip` | Snapshot of persisted queries. |
| `complete_failure` | `-` | not applicable | - | Core state-machine transition, reused from gpui-query's core. Not part of nsh-query's API. |
| `complete_success` | `-` | not applicable | - | Core state-machine transition, reused from gpui-query's core. Not part of nsh-query's API. |
| `contains` | `-` | not applicable | - | Key-filter internals; `QueryFilters` matching covers it. |
| `current_time_ms` | `-` | not applicable | - | Time comes from `nsh_core::time`, not a helper. |
| `dehydrate` | `dehydrate() / dehydrate_with()` | done | `dehydrate_and_hydrate_round_trip` | Only persist-registered queries are saved. |
| `diagnostics` | `QueryClient::diagnostics()`, `use_diagnostics()` | done | `diagnostics_report_queries_and_mutations_after_a_load_and_a_call` | A snapshot of the cache. `use_diagnostics` is the memo a binding follows. |
| `fetch_next_page_infinite` | `InfiniteQuery::fetch_next_page()` | done | `infinite_fetching_flags_and_page_params_follow_tanstack` | Loads the next page. |
| `fetch_previous_page_infinite` | `InfiniteQuery::fetch_previous_page()` | done | `infinite_fetching_flags_and_page_params_follow_tanstack` | Loads the previous page. |
| `fetch_query` | `QueryClient::fetch_query()` | done | `fetch_query_and_ensure_query_data` | Returns `Result<T, FetchQueryError<E>>`. |
| `fetch_query_with_policy` | `QueryClient::fetch_query_with_policy()` | done | `fetch_query_with_policy_applies_the_returned_policy` | The fetcher returns `Fetched`, and its cache policy replaces the entry's on success. |
| `fetch_query_with_signal` | `QueryContext::signal` | done | `fetch_query_and_ensure_query_data` | The fetcher gets the cancellation signal in its context. |
| `force` | `Query::refetch()` | done | `offline_pauses_and_online_resumes` | A forced load, ignoring freshness. |
| `gc` | `QueryOptions::gc()` | done | `stale_time_and_gc_time_synonyms_work` | gc time. |
| `gc_time` | `QueryOptions::gc_time()` | done | `stale_time_and_gc_time_synonyms_work` | Synonym of gc. |
| `gc_with_time` | `-` | not applicable | - | Constructor-style helper on gpui-query's bucket; gc is `QueryOptions::gc`. |
| `get_query_data` | `QueryClient::get_query_data()` | done | `get_query_data_and_get_query_state_read_the_cache` | Cached value. |
| `hydrate` | `hydrate() / hydrate_with()` | done | `dehydrate_and_hydrate_round_trip` | Parks saved values for later queries. |
| `infinite_query` | `QueryClient::infinite_query()` | done | `client_infinite_query_reloads_like_fetch_infinite_query` | Awaitable infinite fetch. |
| `infinite_resource` | `-` | not applicable | - | gpui-query's internal infinite state machine; nsh-query uses it internally. |
| `infinite_resource_with_policies` | `-` | not applicable | - | gpui-query's internal constructor. |
| `invalidate_queries` | `QueryClient::invalidate_queries()` | done | `invalidating_replaces_a_load_in_flight_and_drops_its_late_answer` | Marks stale and reloads observed. |
| `keep_previous` | `QueryOptions::keep_previous_data()` | done | `placeholder_data_and_keep_previous_data_flag_placeholder` | Shows the previous key's data as placeholder. |
| `matches` | `match_query()` | done | `match_query_and_hash_key_follow_tanstack` | Filter matching for a query state. |
| `max_pages` | `InfiniteQueryOptions::max_pages()` | done | `max_pages_drops_from_the_far_end` | Keeps at most this many pages. |
| `mutate` | `Mutation::mutate()` | done | `mutation_pauses_offline_and_resumes` | Starts a call. |
| `mutate_arc` | `Mutation::mutate_arc()` | done | `mutate_by_ref_and_mutate_arc_run_with_their_variables` | Takes the variables from a shared `Arc`, copying them only while the `Arc` is still shared. |
| `mutate_by_ref` | `Mutation::mutate_by_ref()` | done | `mutate_by_ref_and_mutate_arc_run_with_their_variables` | Borrows the variables. The value is cloned once per call, since the mutator takes it by value. |
| `mutate_with_callbacks` | `Mutation::mutate_with()` | done | `mutate_with_callbacks_run_for_the_latest_call_only` | Per-call callbacks. |
| `new` | `-` | not applicable | - | Constructors of gpui-query's internal types; nsh-query builds its own handles. |
| `next_request_id_for_infinite_key` | `-` | not applicable | - | gpui-query's request sequencer, internal to its client. |
| `next_request_id_for_key` | `-` | not applicable | - | gpui-query's request sequencer, internal to its client. |
| `observe` | `-` | not applicable | - | gpui-query's observer; nsh-query observes through `use_query`. |
| `on_error` | `QueryCacheCallbacks::on_error() / MutationCacheCallbacks::on_error()` | done | `query_cache_callbacks_fire_for_success_error_and_settled` | Cache-wide error callback. |
| `on_settled` | `QueryCacheCallbacks::on_settled() / MutationCacheCallbacks::on_settled()` | done | `query_cache_callbacks_fire_for_success_error_and_settled` | Cache-wide settled callback. |
| `on_success` | `QueryCacheCallbacks::on_success() / MutationCacheCallbacks::on_success()` | done | `query_cache_callbacks_fire_for_success_error_and_settled` | Cache-wide success callback. |
| `persist` | `persist_to_storage()` | done | `dehydrate_and_hydrate_round_trip` | Saves to NSH storage. |
| `persist_with` | `persist_to_storage_with()` | done | `persistence_options_select_restore_and_expire` | Saves with a buster. |
| `prepare_fetch_query` | `-` | not applicable | - | gpui-query's internal fetch preparation. |
| `prepare_prefetch_query` | `-` | not applicable | - | gpui-query's internal prefetch preparation. |
| `register` | `-` | not applicable | - | Entries are created by observing or prefetching a key. |
| `register_deserializer` | `QueryConfig::persist()` | done | `dehydrate_and_hydrate_round_trip` | Serde pair set by `persist`. |
| `register_mutation` | `-` | not applicable | - | Mutations register in the mutation cache when created. |
| `register_serializer` | `QueryConfig::persist()` | done | `dehydrate_and_hydrate_round_trip` | Serde pair set by `persist`. |
| `remove_queries` | `QueryClient::remove_queries()` | done | `dehydrate_and_hydrate_round_trip` | Exercised through clear(). |
| `request_policy` | `QueryOptions::request()` | done | `request_policy_latest_wins_replaces_a_load_and_ignore_while_loading_keeps_it` | Both values are checked through a real load. LatestWins cancels the load in flight. |
| `reset_queries` | `QueryClient::reset_queries()` | done | `reset_queries_goes_back_to_initial` | Back to the initial state. |
| `resource` | `-` | not applicable | - | gpui-query's state machine, reused from its core. |
| `resource_with_policies` | `-` | not applicable | - | gpui-query's state machine constructor. |
| `restore` | `restore_from_storage() / restore_from_storage_with()` | done | `persistence_options_select_restore_and_expire` | Hydrates from NSH storage, with max_age and buster. |
| `retry_policy` | `QueryOptions::retry()` | done | `retry_if_stops_early` | Retry policy option. |
| `set_query_data` | `QueryClient::set_query_data()` | done | `update_query_data_uses_the_updater` | Writes a value into the cache. |
| `unbounded_pages` | `InfiniteQueryOptions::unbounded_pages()` | done | `unbounded_pages_keeps_every_page_past_the_default_limit` | No page limit. This is the default too, so more than 50 pages stay. |
| `use_infinite_query` | `use_infinite_query()` | done | `infinite_refetch_reloads_all_pages` | Observes an infinite query. |
| `use_mutation` | `use_mutation()` | done | `mutation_pauses_offline_and_resumes` | Creates a mutation. |
| `use_mutation_state` | `use_mutation_state()` | done | `use_mutation_state_lists_calls` | Mapped states, oldest first. |
| `use_query` | `use_query()` | done | `two_observers_of_one_key_share_one_load` | Observes a key. |
| `use_query_manual` | `use_query_manual()` | done | `use_query_manual_starts_no_fetch_until_asked` | Observes the key and loads nothing on mount or key change. Its fetcher runs on `refetch()`. |
| `use_query_manual_opts` | `use_query_manual_opts()` | done | `use_query_manual_starts_no_fetch_until_asked` | The same, with options. |
| `use_query_select` | `use_query_select()` | done | `use_query_select_binding_wakes_only_when_the_selection_changes` | Returns a memo of the selection. A binding on it re-runs only when the selected value changes. |
| `use_query_unsignalled` | `use_query_unsignalled()` | done | `use_query_unsignalled_loads_with_a_fetcher_that_takes_no_signal` | The fetcher takes no `QueryContext`, so it cannot read the key, the signal or the client. |
| `use_query_unsignalled_opts` | `use_query_unsignalled_opts()` | done | `use_query_unsignalled_loads_with_a_fetcher_that_takes_no_signal` | The same, with options. |
| `use_query_with_policy` | `use_query_with_policy()` | done | `use_query_with_policy_sets_the_freshness_the_server_returns` | The fetcher returns `Fetched`, and its cache policy replaces the default. |
| `with_config` | `-` | not applicable | - | Configures gpui-query's `Observer`, which notifies only when a status or the data changes. Observers are internal here, and every binding already wakes only for what it reads (`tests/fine_grained.rs`). |
| `with_gc_time` | `QueryOptions::gc_time()` | done | `stale_time_and_gc_time_synonyms_work` | gc time builder. |
| `with_policies` | `QueryOptions` | not applicable | - | Constructor of gpui-query's internal policy bundle. |
| `with_query_data` | `QueryClient::with_query_data()` | done | `with_query_data_reads_a_value_that_is_not_clone` | Calls the closure with the data by reference, so the value needs no `Clone`. A binding on it follows that entry's data. |
| `ClientDiagnostic` | `ClientDiagnostic` | done | `diagnostics_report_queries_and_mutations_after_a_load_and_a_call` | Query and mutation counts, and the sorted lists. |
| `DehydratedEntry` | `DehydratedQuery` | done | `dehydrate_and_hydrate_round_trip` | Saved query entry. |
| `DehydratedState` | `DehydratedState` | done | `dehydrate_and_hydrate_round_trip` | Saved state. |
| `DeserializerRegistry` | `-` | not applicable | - | Serde pair is stored per query by `persist`. |
| `InfiniteQueryBucket` | `-` | not applicable | - | gpui-query's internal bucket; nsh-query keeps infinite entries in the client cache. |
| `InfiniteQueryOptions` | `InfiniteQueryOptions` | done | `infinite_refetch_reloads_all_pages` | Infinite options. |
| `MutationBucket` | `-` | not applicable | - | gpui-query's internal bucket; nsh-query keeps calls in the mutation cache. |
| `MutationCallbacks` | `MutateOptions` | done | `mutate_with_callbacks_run_for_the_latest_call_only` | Per-call callbacks. |
| `MutationDiagnostic` | `MutationDiagnostic` | done | `diagnostics_report_queries_and_mutations_after_a_load_and_a_call` | Key, status and retry count of one mutation. |
| `MutationOptions` | `MutationOptions` | done | `mutation_pauses_offline_and_resumes` | Mutation options. |
| `NoopPersister` | `-` | not applicable | - | NSH storage is the persister; the no-op case is a `persist`-less query. |
| `Observer` | `-` | not applicable | - | gpui-query's observer; nsh-query observes through `use_query`. |
| `ObserverConfig` | `-` | not applicable | - | gpui-query's observer config. |
| `PersistHandle` | `-` | not applicable | - | gpui-query-persist's handle; NSH storage is used instead. |
| `PersistOptions` | `PersistOptions` | done | `persistence_options_select_restore_and_expire` | max_age and buster. |
| `PersistSnapshot` | `DehydratedState` | done | `dehydrate_and_hydrate_round_trip` | Snapshot of persisted state. |
| `PersistedEntry` | `DehydratedQuery` | done | `dehydrate_and_hydrate_round_trip` | Saved entry. |
| `PreparedFetch` | `-` | not applicable | - | gpui-query's internal fetch preparation. |
| `QueryBucket` | `-` | not applicable | - | gpui-query's internal bucket; nsh-query keeps entries in the client cache. |
| `QueryClient` | `QueryClient` | done | `query_cache_finds_lists_subscribes_and_removes` | The client. |
| `QueryDiagnostic` | `QueryDiagnostic` | done | `diagnostics_report_queries_and_mutations_after_a_load_and_a_call` | Key, status, cache policy, cache age, cache hits and retry count of one query. |
| `QueryOptions` | `QueryOptions` | done | `stale_time_and_gc_time_synonyms_work` | Query options. |
| `SerializerRegistry` | `-` | not applicable | - | Serde pair is stored per query by `persist`. |
| `CacheMutation` | `-` | not applicable | - | gpui-query's internal mutation cache entry. |
| `ObservableResource` | `-` | not applicable | - | gpui-query's trait for observable state; NSH signals replace it. |
| `Persister` | `-` | not applicable | - | Persister trait: gpui-query-persist's job. NSH storage fills it here. |
| `QueryPersister` | `-` | not applicable | - | Persister trait: gpui-query-persist's job. NSH storage fills it here. |
| `MutationErrorCallback` | `MutationCacheCallbacks::on_error()` | done | `mutation_cache_callbacks_fire_for_each_point_of_a_call` | Type alias for the error callback. |
| `MutationSettledCallback` | `MutationCacheCallbacks::on_settled()` | done | `mutation_cache_callbacks_fire_for_each_point_of_a_call` | Type alias for the settled callback. |
| `MutationSuccessCallback` | `MutationCacheCallbacks::on_success()` | done | `mutation_cache_callbacks_fire_for_each_point_of_a_call` | Type alias for the success callback. |
| `MutationObserver` | `-` | not applicable | - | gpui-query's observer type. |
| `QueryObserver` | `-` | not applicable | - | gpui-query's observer type. |
| `InfiniteQueryObserver` | `-` | not applicable | - | gpui-query's observer type. |
| `QuerySelectResult` | `-` | not applicable | - | gpui-query's select result type; `Query::select` returns a memo. |
| `gpui-query-http: HttpCache, HttpBackend` | `HttpCache, HttpBackend (feature http-cache)` | done | `http_cache_feeds_a_query_through_a_fake_backend` | Re-exported behind the `http-cache` feature. |
| `gpui-query-persist` | `-` | not applicable | - | NSH's Storage fills the file persister's job here, as the brief says. |
