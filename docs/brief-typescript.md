# Brief: nsh-query TypeScript on @tanstack/query-core

Read `docs/brief-common.md` in this worktree first.

## Goal

Replace the hand-written state machine in `packages/nsh-query` with a thin NSH adapter over the real `@tanstack/query-core`, the same way `@tanstack/solid-query` adapts it to Solid. Anyone who knows TanStack Query must be able to use `nsh-query` from the TanStack docs, with the same option names, the same result fields and the same behavior. TanStack's core does the caching, retries, deduplication and garbage collection, so write none of that yourself.

## Your paths

Worktree: `~/fo/.worktrees/nsh-query-tanstack`. You may edit only `packages/nsh-query/**` and `docs/parity-typescript.md`. Delete the old `src/client.ts`, `query.ts`, `mutation.ts`, `infinite.ts` and `policy.ts` once replaced.

## Setup

```bash
cd ~/fo/.worktrees/nsh-query-tanstack/packages/nsh-query
bun add --ignore-scripts --exact @tanstack/query-core@5.107.1 @tanstack/query-persist-client-core@5.107.1
```

Commit-ready means `package.json` and `bun.lock` are correct, and `node_modules` stays ignored. Read every file in `~/fo/.refs/tanstack-query/packages/solid-query/src` before writing code, and port its structure. Do not import `solid-js`.

## Required public API

Export these names from `src/index.ts`:

- Everything from `@tanstack/query-core`, by `export *`, as solid-query does. This gives `QueryClient`, `QueryCache`, `MutationCache`, `focusManager`, `onlineManager`, `notifyManager`, `hashKey`, `keepPreviousData`, `skipToken`, `dehydrate`, `hydrate`, `matchQuery`, `matchMutation`, `CancelledError`, and the rest.
- `useQuery` and its alias `createQuery`. `options` is an accessor, as in solid-query: `createQuery(() => ({ queryKey: ["user", id()], queryFn }), () => client)`. The client argument is optional.
- `useInfiniteQuery` and `createInfiniteQuery`.
- `useMutation` and `createMutation`.
- `useQueries` and `createQueries`, with `combine`.
- `useIsFetching(filters?)`, `useIsMutating(filters?)`, `useMutationState(options?)`.
- `queryOptions`, `infiniteQueryOptions`, `mutationOptions`.
- `QueryClientProvider`, `useQueryClient`, built on `context` from `nsh`, which also mounts and unmounts the client.
- `IsRestoringProvider`, `useIsRestoring`.
- `createStoragePersister()`, and `PersistQueryClientProvider`, in `src/persist.ts`. This persists through NSH's async `Storage` using `@tanstack/query-persist-client-core`.

### The result object

Results read as properties, as in TanStack's docs: `query.data`, `query.isPending`, `mutation.mutate(v)`. Each property is a getter over a memo of that one field, so a binding that reads `query.data` does not rerun when only `query.fetchStatus` changes. A test must prove this with a render counter. Read `query.data` in JSX or a closure, as with any NSH prop.

Every field of `QueryObserverBaseResult` must be there: `data`, `dataUpdatedAt`, `error`, `errorUpdatedAt`, `failureCount`, `failureReason`, `errorUpdateCount`, `isError`, `isFetched`, `isFetchedAfterMount`, `isFetching`, `isLoading`, `isPending`, `isLoadingError`, `isInitialLoading`, `isPaused`, `isPlaceholderData`, `isRefetchError`, `isRefetching`, `isStale`, `isSuccess`, `isEnabled`, `refetch`, `status`, `fetchStatus`.

Infinite queries add `fetchNextPage`, `fetchPreviousPage`, `hasNextPage`, `hasPreviousPage`, `isFetchNextPageError`, `isFetchingNextPage`, `isFetchPreviousPageError`, `isFetchingPreviousPage`.

Mutations give `data`, `variables`, `error`, `isError`, `isIdle`, `isPending`, `isSuccess`, `status`, `mutate`, `mutateAsync`, `reset`, `context`, `failureCount`, `failureReason`, `isPaused`, `submittedAt`.

### NSH wiring

Write these in `src/nsh.ts`. They are the adapter's real work:

- `notifyManager.setBatchNotifyFunction(batch)` with `batch` from `nsh`, so one change is one commit.
- `focusManager.setEventListener`, driven by NSH's `lifecycle()`: focused when `"active"`, not focused when `"background"`. `refetchOnWindowFocus` then means "on return to the foreground". Read `~/fo/.refs/nsh-v0.4.0` for how to observe it from outside a component, and make it idempotent.
- `onlineManager` stays online by default, because 0.4.0 has no network status API. `onlineManager.setOnline(false)` must pause fetches with `networkMode: "online"`, and a test must prove it, including `fetchStatus: "paused"` and the resume.
- Options as an accessor: an `effect` calls `observer.setOptions(defaultedOptions)`, so a tracked `queryKey` moves to the new entry and back to its cache. Port this from solid-query's `useBaseQuery.ts`.
- `cleanup` unsubscribes the observer, so a component that goes stops observing and `gcTime` applies.

### Binding helpers

Remove nothing TanStack provides, and add only NSH-specific things that TanStack cannot have. Do not add a second option vocabulary: the old `key`, `fetch` and `cache` options go away.

## Required tests

Put tests in `tests/` and screens in the root `App.tsx`. Use `{ clock: {} }` everywhere. All of these must exist by name and pass:

1. two observers of one key share one fetch
2. a fresh entry skips the fetch, and an expired `staleTime` refetches on mount
3. `gcTime` drops an unobserved entry
4. retries count in `failureCount` while `status` stays `pending`, then succeed
5. a failure after the last retry gives `status: "error"` and `failureReason`
6. `retry` and `retryDelay` as functions are honored
7. `invalidateQueries` refetches active queries and replaces a fetch in flight, dropping its late answer
8. an `AbortSignal` passed to `fetch` fires when a query is cancelled
9. a tracked key moves to the new entry and back to its cache
10. `enabled` as a tracked value, and `skipToken`
11. `select` and `keepPreviousData`, with `isPlaceholderData`
12. `initialData`, `initialDataUpdatedAt`, `placeholderData`
13. `refetchInterval`, and that it stops in the background unless `refetchIntervalInBackground`
14. `refetchOnWindowFocus` on the `lifecycle` change, using `app.setLifecycle`
15. `onlineManager.setOnline(false)` pauses, and setting it back resumes
16. a binding on `query.data` does not rerun when only `fetchStatus` changes
17. mutation: `onMutate` context reaches `onError` and `onSettled`, for optimistic update and rollback
18. mutation: `invalidateQueries` from `onSuccess`, `mutateAsync` result and rejection, `reset`
19. mutation `retry`, and `scope` running calls in series
20. a mutation in flight finishes after its component goes
21. infinite: `fetchNextPage`, `fetchPreviousPage`, `hasNextPage`, `maxPages`, `pageParams`, refetch of all pages
22. `useQueries` with `combine`, `useIsFetching`, `useIsMutating`, `useMutationState`
23. `setQueryData` with an updater, `getQueryData`, `getQueryState`, `prefetchQuery`, `fetchQuery`, `ensureQueryData`, `removeQueries`, `resetQueries`, `cancelQueries`, `setQueryDefaults`
24. query filters: `exact`, `type`, `stale`, `predicate`
25. a capability refusal (`ERR_ACCESS_DENIED`) reaches `query.error`, and does not retry forever
26. persistence: a query restored from `Storage` after reopening with the same `storage` option, using `dehydrate` and `hydrate`
27. `tests/smoke.test.tsx`: one small end-to-end screen with no options that only a headless run allows

## Docs

- `packages/nsh-query/README.md` shows install through `node_modules`, a query, a mutation, an infinite list and persistence, in TanStack's vocabulary. Say plainly that this is TanStack Query on NSH, and link TanStack's docs as the reference.
- `docs/parity-typescript.md`: a table with one row for every public name of `@tanstack/query-core`'s `index.ts`, every `QueryClient` method, and every option and result field listed above. Columns: name, status (`done`, `not applicable`, or `not done`), the test that covers it, a note. These `QueryClient` methods must each have a row: `cancelQueries`, `clear`, `ensureInfiniteQueryData`, `ensureQueryData`, `fetchInfiniteQuery`, `fetchQuery`, `getDefaultOptions`, `getMutationCache`, `getMutationDefaults`, `getQueriesData`, `getQueryCache`, `getQueryData`, `getQueryDefaults`, `getQueryState`, `infiniteQuery`, `invalidateQueries`, `isFetching`, `isMutating`, `mount`, `prefetchInfiniteQuery`, `prefetchQuery`, `refetchQueries`, `removeQueries`, `resetQueries`, `resumePausedMutations`, `setDefaultOptions`, `setMutationDefaults`, `setQueriesData`, `setQueryData`, `setQueryDefaults`, `unmount`.

## Finish conditions

You are finished only when all of these hold. Run them yourself and paste the output in your report.

1. In `packages/nsh-query`: `nsh check` exits 0.
2. `nsh build` exits 0, and its output has no `warn` line.
3. `nsh test` exits 0, with every test named above present.
4. `grep -rn "new Map\|setTimeout\|retryDelay(" packages/nsh-query/src` finds no hand-written cache, timer or retry logic. Anything left has a stated reason.
5. `nsh build --release` output is recorded in the parity file, with the bundle's size.
6. A consumer works. In a new folder under `/private/tmp/claude-501/-Users-hmziq--claude-spares-spare-NGupus/5378586f-1e26-4b17-89e9-b3eb305cfd38/scratchpad/verify/work/`, run `nsh init . --template ts`, link the package with `mkdir -p node_modules && ln -s ~/fo/.worktrees/nsh-query-tanstack/packages/nsh-query node_modules/nsh-query`, write a test that imports `createQuery` from `"nsh-query"`, and show that `nsh check`, `nsh build` and `nsh test` pass.
7. `nsh test tests/smoke.test.tsx --target macos` passes on a real window.
8. `docs/parity-typescript.md` has every required row, and every row marked `done` names a test that exists.
9. You broke the code once on purpose for each of tests 4, 7, 16 and 21, and saw each fail. Say how in your report.
