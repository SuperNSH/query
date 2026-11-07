# Finish brief: nsh-query TypeScript

Read `docs/brief-finish-common.md` first. You own `packages/nsh-query/**` and `docs/parity-typescript.md`. Another agent owns `crates/`. Keep building on TanStack's own code, and write no caching, retry or persistence logic of your own.

## 1. Close the open rows

Each of these needs a real test: `experimental_streamedQuery`, `unmount`, `networkMode` `"always"` and `"offlineFirst"`, options as a plain object, the client argument as a plain `QueryClient`, `useIsFetching` and `useIsMutating` filters as an accessor, `PersistQueryClientProvider` `buster`, `maxAge`, `dehydrateOptions`, `hydrateOptions`, `createStoragePersister` `key`, `refetchOnMount`, `refetchOnReconnect`, `notifyOnChangeProps`, `meta`, and the `suspense: true` throw.

## 2. Fine-grained state everywhere

`useQuery` is proved by test 16. Add the same proof with render counters for `useMutation`, `useInfiniteQuery` and `useQueries`.

`src/useQueries.ts` says a binding on `queries[i].data` reruns when any result changes. Fix that: per-index, per-field memos made once when the list changes, never inside a binding. A binding on one query's `data` must not wake when another query changes. If `combine` needs the whole value, say so in the note.

## 3. TanStack names not in the first brief

These exist in `react-query`, the fullest adapter. Port each on `nsh`, with a test:

- `usePrefetchQuery`, `usePrefetchInfiniteQuery`
- `HydrationBoundary`
- `QueryErrorResetBoundary`, `useQueryErrorResetBoundary`, and `throwOnError` reaching NSH's `ErrorBoundary`, including a reset that retries
- `useSuspenseQuery`, `useSuspenseInfiniteQuery`, `useSuspenseQueries`: NSH 0.4.0 has no Suspense boundary. Search the installed types first. If it really has none, these rows are `not applicable`, naming that, with the test of the clear error.

Use TanStack's persisters, not a substitute:

- Add `@tanstack/query-async-storage-persister` at exactly 5.107.1, with `bun add --ignore-scripts --exact`, and export `createAsyncStoragePersister`. `createStoragePersister` stays, as a thin call to it over NSH's `Storage`. Test `throttleTime`, `key`, `serialize`, `deserialize` and `retry`.
- Re-export everything from `@tanstack/query-persist-client-core`, including `persistQueryClient`, `persistQueryClientRestore`, `persistQueryClientSave`, `persistQueryClientSubscribe`, `experimental_createQueryPersister`. Test the query persister.

## 4. What `gpui-query` has

Add a second table to `docs/parity-typescript.md`: gpui-query name, TypeScript name, status, test, note. Go through `~/fo/.refs/gpui-query/crates/gpui-query/src/{hook,client}`. Most map to TanStack's names. One that does not:

- `useQueryDiagnostics()`, the same data as gpui-query's `diagnostics()`: for each query its key, status, fetch status, observer count, staleness and last update, and for each mutation its status. It subscribes to the query and mutation caches, ends through `cleanup`, and wakes only when the diagnostics change. TanStack's own devtools are DOM only, so this is the NSH way to build a debug panel. Test it through a screen.

## Docs

Update `packages/nsh-query/README.md` with the new names. Do not edit the repository README; the owner will.

## Finish conditions

1. In `packages/nsh-query`: `nsh check` exits 0.
2. `nsh build` exits 0, with no `warn` line.
3. `nsh test` exits 0.
4. `grep -rn "new Map\|setTimeout\|retryDelay(" packages/nsh-query/src` finds no hand-written cache, timer or retry logic. Anything left has a stated reason.
5. `nsh build --release` output is recorded in the parity file, with the bundle's size.
6. `nsh test tests/smoke.test.tsx --target macos` passes on a real window.
7. `grep -c "| not done" docs/parity-typescript.md` prints 0.
8. Every `done` row in both tables names a test that exists.
9. You broke the code once per group (fine-grained `useQueries`, error boundary, persister, diagnostics), saw the test fail and restored it. Say how.
