# Finish brief: nsh-query Rust

Read `docs/brief-finish-common.md` first. You own `crates/nsh-query/**` and `docs/parity-rust.md`, with the same `CARGO_TARGET_DIR` as before. The root `Cargo.lock` may change when you add a dependency.

## 1. Fine-grained state (do this first)

Today one `Tracked` version signal on each entry wakes every binding that reads that entry. `tests/fine_grained.rs` fails for queries, mutations and infinite queries.

Replace it. One design that fits NSH: each entry holds a small struct of signals under the entry's `create_root` owner (`status`, `fetch_status`, `failure_count`, `error_updated_at`, `data_updated_at`, `is_stale`, and so on), updated together in one `batch` after each change to the core resource. Data with no `PartialEq` gets its own version signal, bumped only when the data changes. Every read method on `Query`, `Mutation` and `InfiniteQuery` subscribes only to its own signal. `use_query_by` must also follow the current entry through its key signal.

`QueryClient::touch()` may remain only for aggregate reads inside a `computed`, such as `use_is_fetching` and `use_is_mutating`, so readers wake only when the result changes. `use_queries` results must be per-index and per-field too. Add a test for that.

## 2. Close the open rows

- `gcTime (mutation)`: implement it. A settled mutation stays listed until `gc_time` passes, then leaves, under an owner that outlives its component.
- `infiniteQuery`: add `QueryClient::infinite_query(..)`, and `query(..)` for TanStack's `client.query(options)`. Read `queryClient.ts` for what they do, including `select`, `enabled` and `staleTime`.

## 3. Reopen these `not applicable` rows

- `getQueryCache`, `getMutationCache`: add `get_query_cache()` and `get_mutation_cache()` with `find`, `find_all`, `get_all`, `subscribe` (returns a stop), `clear` and `remove`. Add the cache-wide callbacks from `queryCache.ts` and `mutationCache.ts`: `on_error`, `on_success`, `on_settled`, and `on_mutate` for mutations, set through a client config.
- `structuralSharing`: add a builder that takes a compare or replace function, as TanStack's option accepts. A refetch whose answer is equal must not wake a `data()` binding. Test it with a counter.
- `skipToken`: mark `done`, mapped to `enabled(false)`, with a test.
- `notifyOnChangeProps`: mark `done`, proved by `tests/fine_grained.rs`.
- `mount`, `unmount`: may stay `not applicable`.

## 4. TanStack names not in the first brief

Add a row and a test for each:

- `mutate_with(variables, MutateOptions)`: per-call `on_success`, `on_error` and `on_settled`, with TanStack's rule that they run only for the latest call and only while the component is alive.
- `mutate_async`: a replaced call must still resolve with its own result, as TanStack's does, instead of `None`.
- `use_prefetch_query`, `use_prefetch_infinite_query`
- `throw_on_error`, on `QueryConfig` and `MutationOptions`, reaching NSH's `catch_error`. Add `use_query_error_reset_boundary` with TanStack's reset behavior.
- `match_query`, `match_mutation`, `hash_key`
- `subscribed(false)`: an observer that does not fetch or follow the entry
- `query_key_hash_fn` if the core allows it, otherwise a note saying why not
- The fetcher context: `query_key`, `meta`, `client`, `signal` or cancellation. For infinite queries also `page_param` and `direction`, and `meta` on `InfiniteQueryOptions`.
- `dehydrate` options `should_dehydrate_query` and `should_dehydrate_mutation`, and `hydrate` options. Persistence options `max_age`, `buster`, `use_is_restoring`.

## 5. Everything `gpui-query` has

Add a second table to `docs/parity-rust.md`: gpui-query name, Rust name, status, test, note. It covers every public item in `~/fo/.refs/gpui-query/crates/gpui-query/src/{hook,client}`. At least:

- `diagnostics()`, `QueryDiagnostic`, `MutationDiagnostic`, `ClientDiagnostic`
- `use_query_manual`, `use_query_manual_opts`, `use_query_select`, `use_query_with_policy`
- `use_query_unsignalled`, `use_query_unsignalled_opts`: read what they are for. If fine-grained reads make them pointless, mark `not applicable`, and say so with a test showing an unread query wakes nothing.
- `mutate_arc`, `mutate_by_ref`, `mutate_with_callbacks`
- `with_query_data`, `all_queries`, `all_infinite_queries`, `all_mutations`
- `register_serializer`, `register_deserializer`, `persist_with`, `collect_persist_snapshot`, `restore`
- `keep_previous`, `force`, `unbounded_pages`, `request_policy`
- `gpui-query-http`: it is on crates.io and depends only on `gpui-query`'s `core`. Add an optional `http-cache` feature that re-exports `HttpCache` and `HttpBackend`, and test it with a fake backend through a query.
- `gpui-query-persist`: its job is a file persister. NSH's `Storage` fills that role here, so `not applicable` with that reason is acceptable.

Reuse the `gpui_query` core. Do not copy its logic.

## Finish conditions

1. `cargo fmt --all -- --check` exits 0.
2. `cargo clippy -p nsh-query --all-targets --all-features -- -D warnings` exits 0.
3. `cargo test -p nsh-query --all-features --no-fail-fast` and `cargo test -p nsh-query --no-fail-fast` exit 0. `tests/query.rs` and `tests/mutation.rs` are unedited, `tests/infinite.rs` has only the owner's `reset()` line, and `tests/fine_grained.rs` is unedited.
4. `grep -n "not done" docs/parity-rust.md` finds no table row.
5. A script confirms that every `done` row in both tables names a test that exists.
6. `grep -rn "version" crates/nsh-query/src` shows no version signal read by a public read method, apart from a data version with its own stated reason.
7. No `std::thread::sleep`, `std::time::Instant` or `SystemTime::now` in `src`.
8. You broke the code once per group (state signals, cache API, per-call callbacks, gpui-query names), saw the test fail and restored it. Say how.
