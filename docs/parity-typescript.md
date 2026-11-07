# TypeScript parity: nsh-query on @tanstack/query-core

Pinned: `@tanstack/query-core` 5.107.1, `@tanstack/query-persist-client-core` 5.107.1 and
`@tanstack/query-async-storage-persister` 5.107.1, on `nsh` 0.4.0. The package is `packages/nsh-query`.

## How to read this file

- `done`: a named test goes through a real UI or the real API and can fail. Each `T` number is in the test
  reference below, and the file asserts that every cited test exists.
- `not applicable`: the concept cannot exist on NSH, and the note names what it depends on. The owner reviews these.
- Type-only names are checked by `nsh check`, which compiles the package and its tests. Each has the test that loads the package (T13).
- Real differences from TanStack are listed in their own section, each with its reason. They are not hidden under `done`.

## Test reference

| Ref | Test (file: title) |
|---|---|
| T1 | `access.test.tsx`: a capability refusal (ERR_ACCESS_DENIED) reaches query.error, and does not retry forever |
| T2 | `access.test.tsx`: persistence: a query restored from Storage after reopening with the same storage option |
| T3 | `access.test.tsx`: PersistQueryClientProvider's onError runs when restoring throws |
| T4 | `direct.test.tsx`: cache helpers: empty, contains, all_queries, all_mutations and all_infinite_queries read the caches |
| T5 | `direct.test.tsx`: with_config, with_gc_time and with_policies: a client keeps the default options it was made with |
| T6 | `direct.test.tsx`: unbounded_pages: an infinite query without maxPages keeps every page |
| T7 | `direct.test.tsx`: a persister over no storage (NoopPersister) saves nothing and restores nothing |
| T8 | `direct.test.tsx`: isServer: the exported constant reads true on NSH, while the core reads environmentManager, which says client |
| T9 | `direct.test.tsx`: useQueryClient outside a provider throws NshQueryError with ERR_NSH_QUERY_NO_CLIENT |
| T10 | `direct.test.tsx`: removeOldestQuery drops the query with the oldest data, and gives up when none are left |
| T11 | `direct.test.tsx`: mutate with per-call callbacks: onSuccess and onSettled given to mutate run after the mutation |
| T12 | `direct.test.tsx`: force: a fetch with staleTime 0 bypasses data that is still fresh |
| T13 | `exports.test.tsx`: every value export of query-core and every adapter name is reachable from the package |
| T14 | `fine.test.tsx`: useMutation: a binding on data does not wake when only status changes |
| T15 | `fine.test.tsx`: useInfiniteQuery: a binding on pages does not wake for a refetch, and does wake for a new page |
| T16 | `fine.test.tsx`: useQueries: a binding on queries[0].data does not wake when another query changes, and the list can grow |
| T17 | `fine.test.tsx`: useQueryDiagnostics reports queries and mutations, and wakes only when they change |
| T18 | `gaps.test.tsx`: unmount: a provider's client stops following focus when the provider goes |
| T19 | `gaps.test.tsx`: experimental_streamedQuery collects the chunks of a stream |
| T20 | `gaps.test.tsx`: networkMode always and offlineFirst run their first attempt offline; online waits |
| T21 | `gaps.test.tsx`: refetchOnMount false keeps stale data on remount; the default refetches |
| T22 | `gaps.test.tsx`: refetchOnReconnect refetches a stale query on reconnect unless turned off |
| T23 | `gaps.test.tsx`: notifyOnChangeProps: a fetch that changes only fetchStatus does not notify the screen |
| T24 | `gaps.test.tsx`: meta reaches the queryFn |
| T25 | `gaps.test.tsx`: suspense: true throws ERR_NSH_QUERY_NO_SUSPENSE to the nearest boundary |
| T26 | `gaps.test.tsx`: useSuspenseQuery, useSuspenseInfiniteQuery and useSuspenseQueries throw the same coded error |
| T27 | `gaps.test.tsx`: options as a plain object, and the client as a plain QueryClient |
| T28 | `gaps.test.tsx`: useIsFetching and useIsMutating take a filter accessor |
| T29 | `gaps.test.tsx`: throwOnError reaches the boundary, and a reset of the boundary retries the query |
| T30 | `gaps.test.tsx`: usePrefetchQuery and usePrefetchInfiniteQuery start the fetch before the query reads it |
| T31 | `gaps.test.tsx`: HydrationBoundary applies dehydrated state before its children read it |
| T32 | `gaps.test.tsx`: after a reset, a query that fails again shows its error as state, without throwing it to the boundary |
| T33 | `hooks.test.tsx`: useQueries with combine, useIsFetching, useIsMutating, useMutationState |
| T34 | `hooks.test.tsx`: setQueryData with an updater, getQueryData, getQueryState, prefetchQuery, fetchQuery, ensureQueryData, removeQueries, resetQueries, cancelQueries, setQueryDefaults |
| T35 | `hooks.test.tsx`: query filters: exact, type, stale, predicate |
| T36 | `hooks.test.tsx`: useQueries without combine reads each query by its index, and maps over them |
| T37 | `hooks.test.tsx`: queryOptions, infiniteQueryOptions and mutationOptions return their options unchanged |
| T38 | `hooks.test.tsx`: throwOnError hands the error to the nearest ErrorBoundary |
| T39 | `hooks.test.tsx`: every field of the query, infinite and mutation results is present |
| T40 | `hooks.test.tsx`: a query takes an explicit client, without a provider around it |
| T41 | `mutations.test.tsx`: onMutate context reaches onError and onSettled, for optimistic update and rollback |
| T42 | `mutations.test.tsx`: invalidateQueries from onSuccess, mutateAsync result and rejection, and reset |
| T43 | `mutations.test.tsx`: mutation retry, and scope running calls in series |
| T44 | `mutations.test.tsx`: a mutation in flight finishes after its component goes |
| T45 | `persist.test.tsx`: buster: a stored client with another buster is discarded, not restored |
| T46 | `persist.test.tsx`: maxAge: an expired stored client is discarded, not restored |
| T47 | `persist.test.tsx`: dehydrateOptions: a query the filter excludes is not stored |
| T48 | `persist.test.tsx`: hydrateOptions: deserializeData transforms restored data |
| T49 | `persist.test.tsx`: createStoragePersister key: the client is saved under the given key only |
| T50 | `persist.test.tsx`: createStoragePersister throttleTime: a second save waits for the interval |
| T51 | `persist.test.tsx`: createStoragePersister serialize and deserialize: a custom format round-trips |
| T52 | `persist.test.tsx`: createAsyncStoragePersister retry: a failed save is tried again, as the retry function asks |
| T53 | `persist.test.tsx`: persistQueryClient saves on change, and its promise resolves after the restore |
| T54 | `persist.test.tsx`: persistQueryClientRestore restores a stored client into a client |
| T55 | `persist.test.tsx`: experimental_createQueryPersister restores a query from storage |
| T56 | `queries.test.tsx`: two observers of one key share one fetch |
| T57 | `queries.test.tsx`: a fresh entry skips the fetch, and an expired staleTime refetches on mount |
| T58 | `queries.test.tsx`: gcTime drops an unobserved entry |
| T59 | `queries.test.tsx`: retries count in failureCount while status stays pending, then succeed |
| T60 | `queries.test.tsx`: a failure after the last retry gives status error and failureReason |
| T61 | `queries.test.tsx`: retry and retryDelay as functions are honored |
| T62 | `queries.test.tsx`: invalidating reloads what is observed and replaces a load in flight, dropping its late answer |
| T63 | `queries.test.tsx`: an AbortSignal passed to fetch fires when a query is cancelled |
| T64 | `queries.test.tsx`: a tracked key moves to the new entry and back to its cache |
| T65 | `queries.test.tsx`: enabled as a tracked value, and skipToken |
| T66 | `queries.test.tsx`: select and keepPreviousData, with isPlaceholderData |
| T67 | `queries.test.tsx`: initialData, initialDataUpdatedAt and placeholderData |
| T68 | `queries.test.tsx`: refetchInterval, and it stops in the background unless refetchIntervalInBackground |
| T69 | `queries.test.tsx`: 13b. an inactive app is still visible, so its refetchInterval keeps running |
| T70 | `queries.test.tsx`: refetchOnWindowFocus on the lifecycle change, using app.setLifecycle |
| T71 | `queries.test.tsx`: onlineManager.setOnline(false) pauses, and setting it back resumes |
| T72 | `queries.test.tsx`: a binding on query.data does not rerun when only fetchStatus changes |
| T73 | `queries.test.tsx`: pages load in both directions, stop at the ends, honor maxPages, and refetch every page |
| T74 | `smoke.test.tsx`: the smoke screen renders with no options |

## Decisions and measured facts

1. **NSH has no `window`, so query-core treats it as a server.** Measured before the fix: default `retry` was 0 and
   `refetchInterval` never fired. `src/nsh.ts` calls `environmentManager.setIsServer(() => false)`. T1 proves the default retry of 3 (4 attempts).
2. **Focus follows NSH's lifecycle while a `QueryClientProvider` is shown.** `active`, and `inactive` (visible without the
   focus), count as focused. Only `background` does not. This matches query-core's own check, which treats only a hidden page as
   unfocused (`visibilityState !== "hidden"`). The effect is made inside the provider, so it ends with the provider (T18 proves it).
3. **Settings are module-level, subscriptions are not.** `environmentManager` and `notifyManager`'s batch function are set
   once in `src/nsh.ts`. They are configuration, not subscriptions, so they have no owner to end with.
4. **Notifications run in one NSH `batch`.** `notifyManager.setBatchNotifyFunction(batch)`: one change, one update.
5. **`structuralSharing` is left at its default.** An unchanged `data` keeps its identity, so the per-field memos in T72 stop a binding from rerunning.
6. **Result fields are getters over one memo per field** (`src/result.ts`). A binding on `data` reruns only when `data` changes.
7. **Suspense is not available.** See the suspense rows (`not applicable`).
8. **The client is read once per component.** A different client passed later is not picked up.
9. **Subscription waits for restore.** While `PersistQueryClientProvider` restores, no observer subscribes, because subscribing fetches on mount.
10. **`useQueries` has two result shapes.** Without `combine`, `queries[i]` is a view with one memo per index and field, made in an effect
    when the list length changes, and read through a signal of views. With `combine`, reads follow the whole combined value.
    Whether `combine` is given is read once, when the hook is called.
11. **Persistence stores one JSON value** through TanStack's async-storage persister over NSH `Storage` (5 MB). `throttleTime` defaults to 1000 ms.
12. **`invalidateQueries` does not replace a first load that has no data yet.** That is query-core's rule: `cancelRefetch` only cancels a fetch that has data to keep. T62 loads data first.
13. **`App.tsx` is the test harness.** It holds one screen per test that needs the UI, reached from a home list that hides once a screen is chosen.
    Its module-level signals (for example the log) belong to the harness, not to the library.

## Reactivity: what each public read subscribes to

The state standards from `docs/brief-finish-common.md`. For each public read: the owner (the component or effect that holds
the subscription), the signal or memo the read depends on, and the test that fails if that dependency regresses.

| Public read | Owner | Subscribes to | Wakes when | Test (fails if it regresses) |
|---|---|---|---|---|
| `query.<field>` (useQuery, useInfiniteQuery) | component body (`reactiveResult`) | one memo per field, over the observer result signal `current` | that field's value changes; an equal value wakes nothing | T72 |
| `mutation.<field>` (useMutation) | component body (`reactiveResult`) | one memo per field, over the mutation result signal | that field's value changes | T14 |
| `infinite.<field>` (useInfiniteQuery) | component body (`reactiveResult`) | one memo per field, over the observer result signal | that field's value changes | T15 |
| `queries[i].<field>` (useQueries, no combine) | effect owned by the component; rebuilt when the list length changes | a memo per index and field over the combined-result signal; the list reads a signal of views | that query's field changes; a list-length change rebuilds the views | T16 |
| `useQueries` with `combine` | component body | the combined-result signal `current` | any change of the combined value (documented: whole-value reads) | T33 |
| `useIsFetching()` / `useIsMutating()` | component body; cache subscription ends in `cleanup` | a count signal; filters read by an effect | the count changes | T28 |
| `useMutationState()` | component body; cache subscription ends in `cleanup` | a signal of mutation states, replaced only when `replaceEqualDeep` finds a change | the selected states change | T33 |
| `useQueryDiagnostics()` | component body; cache subscriptions end in `cleanup` | a snapshot signal, replaced only when `replaceEqualDeep` finds a change | a diagnostic changes; a no-change cache event wakes nothing | T17 |
| `useQueryErrorResetBoundary().isReset()` | `QueryErrorResetBoundary` body | a reset signal, read by the query's throw effect | `reset()` is called | T29, T32 |
| `useIsRestoring()` | `PersistQueryClientProvider` body | a restoring signal, read by queries before they subscribe | the restore finishes | T2 |
| Focus (lifecycle) | `QueryClientProvider` body: effect ends with the provider | NSH `lifecycle()` | the app moves between active, inactive and background | T70, T18 |
| `usePrefetchQuery` / `usePrefetchInfiniteQuery` | component body: effect over the options accessor | the options accessor | the options change (the fetch starts again) | T30 |
| `HydrationBoundary` | component body; effect over `props.state` | `props.state` | a new state object arrives (hydrated before children render on mount) | T31 |
| `createStoragePersister()` | none: a persister holds no signals | NSH `Storage` calls, in order | n/a | T49 |

Cleanup: every subscription has its stop. `useBaseQuery`, `useMutation`, `useQueries`, `useIsFetching`, `useIsMutating`,
`useMutationState` and `useQueryDiagnostics` return their unsubscribe through `cleanup`. The focus effect ends with its provider.
`PersistQueryClientProvider` stops its subscription in `cleanup`, and does not subscribe if it went before the restore finished.

Changed to meet the standards, each with the test that fails if it regresses:

- `useQueries`: per-index memos were made inside a binding, where they stop updating after the binding's first run. They are
  now made in an effect when the length changes, and the bindings read a signal of views. Regression test: T16 (the break that fails it is recorded in the verification log).
- `useQueryErrorResetBoundary`: a reset now stops a query error from being thrown again. Regression test: T32.
- `useQueryDiagnostics`: a snapshot identical in content does not replace the signal. Regression test: T17.
- The focus listener moved from module load into `QueryClientProvider`, so its effect has an owner. Regression test: T18.

## Real differences from TanStack Query (kept listed, with reasons)

- Focus tracking belongs to `QueryClientProvider`. With two providers, when one goes the other stops following the app's focus. An effect made under a provider cannot outlive it, and most apps have one provider.

| Difference | Reason | Test that pins it |
|---|---|---|
| `isServer` (the exported constant) reads `true`; the core's behaviour follows `environmentManager`, which says client | The constant reads `typeof window`, and NSH has no window. The package sets `environmentManager` so the core behaves as a client (decision 1). | T8 |
| Focus counts `inactive` (visible without the focus) as focused | Matches query-core's own visibility check; NSH's docs say `inactive` is still visible (decision 2). | T69 |
| Online is always true unless `onlineManager.setOnline(false)` is called | NSH 0.4.0 has no network status API. | T71 |
| Suspense hooks and `suspense: true` throw `ERR_NSH_QUERY_NO_SUSPENSE` | NSH 0.4.0 has no Suspense boundary (see the not applicable rows). | T25 |
| `combine` presence is read once, when the hook is called | The list views are built for one shape. Changing it later would need a different component. | T36 |
| The client is read once per component | The hooks read the client in the component body, as NSH expects; a new client needs a new component. | T40 |
| Persistence is one JSON value, capped by NSH `Storage` at 5 MB | NSH `Storage` is key-value with a 5 MB limit; a larger save fails and `retry` decides. | T52 |
| `StoragePersisterOptions` names the package's own options type, shadowing the core's of the same name | The package re-exports `@tanstack/query-persist-client-core` with `export *`. Its `StoragePersisterOptions` is the core's async-storage shape; the package's name here is its own, from `createStoragePersister`. | T49 |
| gpui-query diagnostic names differ: `retry_count` is `failureCount`, and `query_count` / `mutation_count` are `.length` of the lists | TanStack's own field names, and a snapshot of plain data. | T17 |
| The focus listener is global: the last `QueryClientProvider` shown owns it | `focusManager` is a single module object in query-core, so only one client at a time follows the lifecycle. | T18 |

## TanStack: query-core value exports

The value exports of `@tanstack/query-core` `src/index.ts`, from its explicit export blocks. Each is reachable from the package (T13).

| Name | Status | Test | Note |
|---|---|---|---|
| `focusManager` | done | T70, T69 | Installed by `QueryClientProvider` through `setEventListener`, driven by `lifecycle()`. |
| `environmentManager` | done | T68, T1 | `src/nsh.ts` sets `setIsServer(() => false)`; decision 1. |
| `defaultShouldDehydrateMutation` | done | T53 | Default filter of `dehydrate` in the persistence save. |
| `defaultShouldDehydrateQuery` | done | T53 | Default filter of `dehydrate` in the persistence save. |
| `dehydrate` | done | T53, T31 | Used by the save, and by the hydration boundary's test state. |
| `dehydrateQuery` | done | T53 | Called inside `dehydrate`. |
| `hydrate` | done | T31, T54 | Used by the restore and by `HydrationBoundary`. |
| `InfiniteQueryObserver` | done | T73 | The observer behind `useInfiniteQuery`. |
| `MutationCache` | done | T33, T4 | `useMutationState` and `useIsMutating` subscribe to it. |
| `MutationObserver` | done | T41, T11 | The observer behind `useMutation`. |
| `defaultScheduler` | done | T56, T58 | Kept. Every timer test runs through it on the manual clock. |
| `notifyManager` | done | T56 | `setBatchNotifyFunction(batch)` in `src/nsh.ts`; decision 4. |
| `onlineManager` | done | T71 | `setOnline(false)` pauses; `setOnline(true)` resumes. |
| `QueriesObserver` | done | T33, T36 | The observer behind `useQueries`. |
| `QueryCache` | done | T33, T35 | `useIsFetching` subscribes; `findAll` runs the filters. |
| `QueryClient` | done | T56 | Every test that opens the app uses it. |
| `QueryObserver` | done | T56 | The observer behind `useQuery`; `unmount` drives it directly. |
| `CancelledError` | done | T63 | Thrown when a fetch is cancelled; the abort test drives the cancel. |
| `isCancelledError` | done | T63 | The cancel path that the abort test drives checks it. |
| `timeoutManager` | done | T57, T58, T68 | Its default provider is the global timers, which the manual clock drives. |
| `hashKey` | done | T56, T64 | Key sharing depends on query-core's hashing. |
| `isServer` | done | T8 | Real difference (listed above): the constant reads true on NSH; the core follows `environmentManager`. |
| `keepPreviousData` | done | T66 | The `placeholderData` function in that test. |
| `matchMutation` | done | T4 | Used by `MutationCache.findAll`; the helper test filters the mutation cache. |
| `matchQuery` | done | T35 | Used by `QueryCache.findAll`. |
| `noop` | done | T41 | `useMutation` passes it to `catch` so `mutate` never rejects. |
| `partialMatchKey` | done | T35 | The partial `queryKey` match. |
| `replaceEqualDeep` | done | T17 | The diagnostics and mutation-state snapshots keep their identity through it. |
| `shouldThrowError` | done | T38 | Decides `throwOnError` in the adapter. |
| `skipToken` | done | T65 | Disables a query by its `queryFn`. |
| `experimental_streamedQuery` | done | T19 | The streamed query collects the stream's chunks. |
| `Mutation` | done | T41 | The mutation object the mutation cache holds. |
| `Query` | done | T56 | The query object the query cache holds. |

## TanStack: query-core type and constant exports

Types are compiled by `nsh check`, which type-checks the package and its tests. The three constants are reachable (T13).

| Name | Status | Test | Note |
|---|---|---|---|
| `FocusManager` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationCacheNotifyEvent` | done | nsh check (types); T13 | Type-only re-export. |
| `OnlineManager` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryCacheNotifyEvent` | done | nsh check (types); T13 | Type-only re-export. |
| `ManagedTimerId` | done | nsh check (types); T13 | Type-only re-export. |
| `TimeoutCallback` | done | nsh check (types); T13 | Type-only re-export. |
| `TimeoutManager` | done | nsh check (types); T13 | Type-only re-export. |
| `TimeoutProvider` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationFilters` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryFilters` | done | nsh check (types); T13 | Type-only re-export. |
| `SkipToken` | done | nsh check (types); T13 | Type-only re-export. |
| `Updater` | done | nsh check (types); T13 | Type-only re-export. |
| `DehydratedState` | done | nsh check (types); T13 | Type-only re-export. |
| `DehydrateOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `HydrateOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationState` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationCacheConfig` | done | nsh check (types); T13 | Type-only re-export. |
| `QueriesObserverOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryState` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryCacheConfig` | done | nsh check (types); T13 | Type-only re-export. |
| `NonUndefinedGuard` | done | nsh check (types); T13 | Type-only re-export. |
| `DistributiveOmit` | done | nsh check (types); T13 | Type-only re-export. |
| `OmitKeyof` | done | nsh check (types); T13 | Type-only re-export. |
| `Override` | done | nsh check (types); T13 | Type-only re-export. |
| `Register` | done | nsh check (types); T13 | Type-only re-export. |
| `DefaultError` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryKey` | done | nsh check (types); T13 | Type-only re-export. |
| `dataTagSymbol` | done | T13 | Constant symbol re-exported by `export *`; reachable from the package. |
| `dataTagErrorSymbol` | done | T13 | Constant symbol re-exported by `export *`; reachable from the package. |
| `unsetMarker` | done | T13 | Constant symbol re-exported by `export *`; reachable from the package. |
| `UnsetMarker` | done | nsh check (types); T13 | Type-only re-export. |
| `AnyDataTag` | done | nsh check (types); T13 | Type-only re-export. |
| `DataTag` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryKeyWithDataTag` | done | nsh check (types); T13 | Type-only re-export. |
| `InferDataFromTag` | done | nsh check (types); T13 | Type-only re-export. |
| `InferErrorFromTag` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `StaleTime` | done | nsh check (types); T13 | Type-only re-export. |
| `StaleTimeFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryBooleanOption` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryPersister` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryFunctionContext` | done | nsh check (types); T13 | Type-only re-export. |
| `InitialDataFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `PlaceholderDataFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `QueriesPlaceholderDataFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryKeyHashFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `GetPreviousPageParamFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `GetNextPageParamFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteData` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryMeta` | done | nsh check (types); T13 | Type-only re-export. |
| `NetworkMode` | done | nsh check (types); T13 | Type-only re-export. |
| `NotifyOnChangeProps` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `InitialPageParam` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryPageParamsOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `ThrowOnError` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `WithRequired` | done | nsh check (types); T13 | Type-only re-export. |
| `DefaultedQueryObserverOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `DefaultedInfiniteQueryObserverOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryExecuteOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `FetchQueryOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `EnsureQueryDataOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `EnsureInfiniteQueryDataOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryExecuteOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `FetchInfiniteQueryOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `ResultOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `RefetchOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `InvalidateQueryFilters` | done | nsh check (types); T13 | Type-only re-export. |
| `RefetchQueryFilters` | done | nsh check (types); T13 | Type-only re-export. |
| `InvalidateOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `ResetOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `FetchNextPageOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `FetchPreviousPageOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryStatus` | done | nsh check (types); T13 | Type-only re-export. |
| `FetchStatus` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverBaseResult` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverPendingResult` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverLoadingResult` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverLoadingErrorResult` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverRefetchErrorResult` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverSuccessResult` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverPlaceholderResult` | done | nsh check (types); T13 | Type-only re-export. |
| `DefinedQueryObserverResult` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryObserverResult` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverBaseResult` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverPendingResult` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverLoadingResult` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverLoadingErrorResult` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverRefetchErrorResult` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverSuccessResult` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverPlaceholderResult` | done | nsh check (types); T13 | Type-only re-export. |
| `DefinedInfiniteQueryObserverResult` | done | nsh check (types); T13 | Type-only re-export. |
| `InfiniteQueryObserverResult` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationKey` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationStatus` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationScope` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationMeta` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationFunctionContext` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationObserverOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `MutateOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `MutateFunctionRest` | done | nsh check (types); T13 | Type-only re-export. |
| `MutateFunction` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationObserverBaseResult` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationObserverIdleResult` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationObserverLoadingResult` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationObserverErrorResult` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationObserverSuccessResult` | done | nsh check (types); T13 | Type-only re-export. |
| `MutationObserverResult` | done | nsh check (types); T13 | Type-only re-export. |
| `QueryClientConfig` | done | nsh check (types); T13 | Type-only re-export. |
| `DefaultOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `CancelOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `SetDataOptions` | done | nsh check (types); T13 | Type-only re-export. |
| `NotifyEventType` | done | nsh check (types); T13 | Type-only re-export. |
| `NotifyEvent` | done | nsh check (types); T13 | Type-only re-export. |

## TanStack: nsh-query names

| Name | Status | Test | Note |
|---|---|---|---|
| `useQuery` | done | T56 | Options as an accessor, or a plain object. |
| `createQuery` | done | T56 | Alias of `useQuery`. |
| `useInfiniteQuery` | done | T73 | Adds the infinite result fields. |
| `createInfiniteQuery` | done | T73 | Alias of `useInfiniteQuery`. |
| `useMutation` | done | T41 | `mutate` never rejects; `mutateAsync` does. |
| `createMutation` | done | T41 | Alias of `useMutation`. |
| `useQueries` | done | T33, T36 | With and without `combine`. |
| `createQueries` | done | T33 | Alias of `useQueries`. |
| `useIsFetching` | done | T28 | Plain or accessor filters. |
| `useIsMutating` | done | T28 | Plain or accessor filters. |
| `useMutationState` | done | T33 | Accessor of the selected mutation states. |
| `queryOptions` | done | T37 | Returns its argument unchanged, typed for `useQuery`. |
| `infiniteQueryOptions` | done | T37 | Returns its argument unchanged. |
| `mutationOptions` | done | T37 | Returns its argument unchanged. |
| `QueryClientProvider` | done | T56 | Mounts the client, and follows the lifecycle, while shown (decision 2). |
| `useQueryClient` | done | T34, T9 | The provider's client; throws `NshQueryError` when there is none. |
| `IsRestoringProvider` | done | T2 | Provided by `PersistQueryClientProvider` while the restore runs. |
| `useIsRestoring` | done | T2 | Queries wait while it reads true (decision 9). |
| `QueryErrorResetBoundary` | done | T29, T32 | Lets a boundary's reset retry queries. |
| `useQueryErrorResetBoundary` | done | T29, T32 | Returns the reset value; a no-op when there is no boundary. |
| `usePrefetchQuery` | done | T30 | Starts `prefetchQuery` while shown. |
| `usePrefetchInfiniteQuery` | done | T30 | Starts `prefetchInfiniteQuery` while shown. |
| `HydrationBoundary` | done | T31 | Hydrates before the children render. |
| `useSuspenseQuery` | not applicable | T26 | Depends on a Suspense boundary, which NSH 0.4.0 does not have (its installed types export none; `resource()` is NSH's async value). Calling it throws `ERR_NSH_QUERY_NO_SUSPENSE`. |
| `useSuspenseInfiniteQuery` | not applicable | T26 | Depends on a Suspense boundary, which NSH 0.4.0 does not have. Throws `ERR_NSH_QUERY_NO_SUSPENSE`. |
| `useSuspenseQueries` | not applicable | T26 | Depends on a Suspense boundary, which NSH 0.4.0 does not have. Throws `ERR_NSH_QUERY_NO_SUSPENSE`. |
| `useQueryDiagnostics` | done | T17 | Snapshot of queries and mutations; wakes only on change. |
| `NshQueryError` | done | T9, T25 | Error with a stable `code`. |
| `errorCodes` | done | T9, T25 | The codes `NshQueryError` carries. |
| `createStoragePersister` | done | T49, T50, T51 | TanStack's async-storage persister over NSH `Storage`. |
| `PersistQueryClientProvider` | done | T2, T3, T45, T46 | Restores, then saves on change. |
| `UseQueryOptions` | done | nsh check (types); T13 | Type-only export. |
| `DefinedInitialDataOptions` | done | nsh check (types); T13 | Type-only export. |
| `UndefinedInitialDataOptions` | done | nsh check (types); T13 | Type-only export. |
| `UseInfiniteQueryOptions` | done | nsh check (types); T13 | Type-only export. |
| `UseMutationOptions` | done | nsh check (types); T13 | Type-only export. |
| `UseMutateFunction` | done | nsh check (types); T13 | Type-only export. |
| `UseMutationResult` | done | nsh check (types); T13 | Type-only export. |
| `QueriesResults` | done | nsh check (types); T13 | Type-only export. |
| `QueryResultFor` | done | nsh check (types); T13 | Type-only export. |
| `UseQueriesOptions` | done | nsh check (types); T13 | Type-only export. |
| `MutationStateOptions` | done | nsh check (types); T13 | Type-only export. |
| `StoragePersisterOptions` | done | nsh check (types); T13 | Type-only export. |
| `PersistQueryClientProviderProps` | done | nsh check (types); T13 | Type-only export. |
| `QueryClientProviderProps` | done | nsh check (types); T13 | Type-only export. |
| `QueryErrorResetValue` | done | nsh check (types); T13 | Type-only export. |
| `HydrationBoundaryProps` | done | nsh check (types); T13 | Type-only export. |
| `QueryDiagnostics` | done | nsh check (types); T13 | Type-only export. |
| `QueryDiagnosticEntry` | done | nsh check (types); T13 | Type-only export. |
| `MutationDiagnosticEntry` | done | nsh check (types); T13 | Type-only export. |
| `PersistQueryClientProviderProps` | done | nsh check (types); T13 | Type-only export. |
| `AsyncPersistRetryer` | done | nsh check (types); T13 | Type-only export. |

## TanStack: persistence re-exports (`@tanstack/query-persist-client-core`, `@tanstack/query-async-storage-persister`)

| Name | Status | Test | Note |
|---|---|---|---|
| `persistQueryClient` | done | T53, T54 | Returns `[unsubscribe, restorePromise]`; the promise resolves after the restore. |
| `persistQueryClientRestore` | done | T54 | Restores into a client. |
| `persistQueryClientSave` | done | T53 | Called by the subscription on each cache change. |
| `persistQueryClientSubscribe` | done | T2 | Used by `PersistQueryClientProvider` after the restore. |
| `experimental_createQueryPersister` | done | T55 | Per-query persister, given as the query's `persister` option. |
| `removeOldestQuery` | done | T10 | Retry strategy: drops the query with the oldest data. |
| `PERSISTER_KEY_PREFIX` | done | T55 | The `tanstack-query-` prefix of the per-query keys. |
| `createAsyncStoragePersister` | done | T52, T7 | Re-exported from the async-storage persister package. |
| `Persister` | done | T53 | Type-only; the persister interface. |
| `PersistedClient` | done | T54 | Type-only; the saved shape. |
| `PersistedQuery` | done | T55 | Type-only; the per-query saved shape. |
| `PersistQueryClientOptions` | done | T53 | Type-only. |
| `PersistQueryClientRootOptions` | done | T53 | Type-only. |
| `PersistedQueryClientRestoreOptions` | done | T54 | Type-only. |
| `PersistedQueryClientSaveOptions` | done | T53 | Type-only. |
| `PersistRetryer` | done | T10 | Type-only; the retry strategy shape. |
| `MaybePromise` | done | T53 | Type-only. |
| `Promisable` | done | T53 | Type-only. |
| `AsyncStorage` | done | T52 | Type-only; the storage shape NSH `Storage` satisfies. |

## TanStack: QueryClient methods

| Name | Status | Test | Note |
|---|---|---|---|
| `cancelQueries` | done | T63, T34 | Cancels an in-flight load. |
| `clear` | done | T56 | The app's home handler clears the client before each screen. |
| `ensureInfiniteQueryData` | done | T34 | Logged in the client actions. |
| `ensureQueryData` | done | T34 | Returns the cached value without calling `queryFn`. |
| `fetchInfiniteQuery` | done | T34, T6 | Used in the client actions and the unbounded test. |
| `fetchQuery` | done | T34, T12 | Fresh data is not fetched again; `staleTime: 0` fetches. |
| `getDefaultOptions` | done | T34, T5 | Returns the defaults. |
| `getMutationCache` | done | T33 | `useMutationState` reads it. |
| `getMutationDefaults` | done | T34 | Logged in the client actions. |
| `getQueriesData` | done | T34 | Logged in the client actions. |
| `getQueryCache` | done | T35, T4 | Read by the filter test and the cache-helper test. |
| `getQueryData` | done | T41, T34 | Read in the rollback and the client actions. |
| `getQueryDefaults` | done | T34 | Logged in the client actions. |
| `getQueryState` | done | T34 | Logged in the client actions. |
| `infiniteQuery` | done | T34 | Logged in the client actions. |
| `invalidateQueries` | done | T62, T42 | Refetches observed queries; replaces a load in flight. |
| `isFetching` | done | T34 | Logged in the client actions. |
| `isMutating` | done | T34 | Logged in the client actions. |
| `mount` | done | T70, T71 | Called by `QueryClientProvider`; the focus and online listeners it subscribes are what those tests use. |
| `prefetchInfiniteQuery` | done | T34 | Logged in the client actions. |
| `prefetchQuery` | done | T34, T30 | Logged in the client actions and used by the prefetch hook. |
| `refetchQueries` | done | T62, T34 | Starts the slow refetch in the invalidation test. |
| `removeQueries` | done | T34 | Logged in the client actions. |
| `resetQueries` | done | T34 | Logged in the client actions. |
| `resumePausedMutations` | done | T34 | Logged in the client actions. |
| `setDefaultOptions` | done | T34 | Logged in the client actions. |
| `setMutationDefaults` | done | T34 | Logged in the client actions. |
| `setQueriesData` | done | T34 | Logged in the client actions. |
| `setQueryData` | done | T34, T44 | Updater form; optimistic write; a write after an unmount. |
| `setQueryDefaults` | done | T34 | Logged in the client actions. |
| `unmount` | done | T18 | Called when the provider goes; the test shows the client no longer refetches on focus. |
| `constructor` | done | T5 | Creates the client with defaults. |
| `defaultQueryOptions` | done | T5 | Applied to each options object. |
| `defaultMutationOptions` | done | T41, T43 | Applied to each mutation. |

## TanStack: options

| Name | Status | Test | Note |
|---|---|---|---|
| `queryKey` | done | T56, T64 | Tracked keys move entries. |
| `queryFn` | done | T56 | Receives `signal`, `queryKey` and `meta`. |
| `staleTime` | done | T57 | Fresh entries skip the fetch; expired ones refetch on mount. |
| `gcTime` | done | T58 | An unobserved entry is dropped after the time. |
| `retry` | done | T59, T61, T43, T1 | Numbers and functions; the default of 3 (four attempts). |
| `retryDelay` | done | T59, T61, T43 | Numbers and functions. |
| `retryOnMount` | done | T1 | Default `true`: an errored query retries when it mounts (the reset test refetches on remount). |
| `enabled` | done | T65 | A tracked value. |
| `skipToken` | done | T65 | As `queryFn`. |
| `select` | done | T66 | Transforms `data`; the cache keeps the original. |
| `placeholderData` | done | T66, T67 | Value and `keepPreviousData` function forms. |
| `keepPreviousData` | done | T66 | With `isPlaceholderData`. |
| `initialData` | done | T67 | Value form. |
| `initialDataUpdatedAt` | done | T67 | Stale and fresh seeds. |
| `refetchInterval` | done | T68 | Runs on the manual clock; stops in the background. |
| `refetchIntervalInBackground` | done | T68, T69 | The background poll keeps running. |
| `refetchOnWindowFocus` | done | T70 | On return to foreground; `false` stays put. |
| `refetchOnMount` | done | T21 | `false` keeps stale data on remount; the default refetches. |
| `refetchOnReconnect` | done | T22 | Refetches on reconnect; `false` stays put. |
| `notifyOnChangeProps` | done | T23 | Only the named props notify: a fetch that changes only `fetchStatus` leaves the screen unchanged. |
| `meta` | done | T24 | Reaches the `queryFn` context. |
| `networkMode: "online"` | done | T71 | The default; pauses while offline. |
| `networkMode: "always"` | done | T20 | Runs offline. |
| `networkMode: "offlineFirst"` | done | T20 | The first attempt runs offline. |
| `throwOnError` | done | T38, T29 | The error reaches the nearest `ErrorBoundary`. |
| `suspense` | done | T25 | `suspense: true` throws `ERR_NSH_QUERY_NO_SUSPENSE` (see the suspense rows). |
| `maxPages` | done | T73 | Drops the far page when a page is added. |
| `initialPageParam` | done | T73 | The first page. |
| `getNextPageParam` | done | T73 | `hasNextPage` and `fetchNextPage`. |
| `getPreviousPageParam` | done | T73 | `hasPreviousPage` and `fetchPreviousPage`. |
| `persister (query option)` | done | T55 | The per-query persister. |
| `scope` | done | T43 | Mutations with the same scope run in series. |
| `retry (mutation)` | done | T43 | A mutation fails twice, then succeeds. |
| `retryDelay (mutation)` | done | T43 | The fixed delay. |
| `mutationFn` | done | T41 | Every mutation test. |
| `onMutate` | done | T41 | Its return value is the context. |
| `onError` | done | T41 | Receives the context and rolls back. |
| `onSettled` | done | T41 | Receives the context. |
| `onSuccess` | done | T42, T44 | Invalidates queries; sets query data after an unmount. |
| `mutate per-call callbacks (onSuccess, onSettled)` | done | T11 | Run while the observer has a listener. |
| `combine` | done | T33 | The combined value is what the component reads. |
| `options as an accessor` | done | T56 | Tracked options move the query. |
| `options as a plain object` | done | T27 | Read once; not tracked. |
| `client argument as an accessor` | done | T40 | `() => client`, with no provider around the query. |
| `client argument as a plain QueryClient` | done | T27 | The client is passed directly. |
| `useIsFetching / useIsMutating filters as an accessor` | done | T28 | Accessor filters track their signals. |
| `persistOptions: persister` | done | T2 | The NSH `Storage` persister. |
| `persistOptions: buster` | done | T45 | A different buster discards the stored client. |
| `persistOptions: maxAge` | done | T46 | An expired stored client is discarded. |
| `persistOptions: dehydrateOptions` | done | T47 | `shouldDehydrateQuery` excludes a query from storage. |
| `persistOptions: hydrateOptions` | done | T48 | `defaultOptions.deserializeData` transforms restored data. |
| `PersistQueryClientProvider onSuccess` | done | T2 | Logged after the restore. |
| `PersistQueryClientProvider onError` | done | T3 | Logged when the restore throws. |
| `createStoragePersister key` | done | T49 | The custom key holds the client. |
| `createStoragePersister throttleTime` | done | T50 | A second save waits for the interval. |
| `createStoragePersister serialize / deserialize` | done | T51 | A custom format round-trips. |
| `createAsyncStoragePersister retry` | done | T52 | The retry function decides whether to save again. |

## TanStack: result fields

Each field is on the result object (T39 checks every name with `in`), and reads go through the memo of that field (decision 6).

| Name | Status | Test | Note |
|---|---|---|---|
| `data` | done | T72, T39 | Present; read through its own memo. |
| `dataUpdatedAt` | done | T39 | Present; read through its own memo. |
| `error` | done | T39 | Present; read through its own memo. |
| `errorUpdatedAt` | done | T39 | Present; read through its own memo. |
| `failureCount` | done | T59 | Present; read through its own memo. |
| `failureReason` | done | T60 | Present; read through its own memo. |
| `errorUpdateCount` | done | T39 | Present; read through its own memo. |
| `isError` | done | T39 | Present; read through its own memo. |
| `isFetched` | done | T39 | Present; read through its own memo. |
| `isFetchedAfterMount` | done | T39 | Present; read through its own memo. |
| `isFetching` | done | T39 | Present; read through its own memo. |
| `isLoading` | done | T39 | Present; read through its own memo. |
| `isPending` | done | T39 | Present; read through its own memo. |
| `isLoadingError` | done | T39 | Present; read through its own memo. |
| `isInitialLoading` | done | T39 | Present; read through its own memo. |
| `isPaused` | done | T39 | Present; read through its own memo. |
| `isPlaceholderData` | done | T66 | Present; read through its own memo. |
| `isRefetchError` | done | T39 | Present; read through its own memo. |
| `isRefetching` | done | T39 | Present; read through its own memo. |
| `isStale` | done | T39 | Present; read through its own memo. |
| `isSuccess` | done | T39 | Present; read through its own memo. |
| `isEnabled` | done | T39 | Present; read through its own memo. |
| `refetch` | done | T71, T72 | Present; read through its own memo. |
| `status` | done | T59, T60 | Present; read through its own memo. |
| `fetchStatus` | done | T72, T71 | Present; read through its own memo. |
| `fetchNextPage` | done | T73 | Present; read through its own memo. |
| `fetchPreviousPage` | done | T73 | Present; read through its own memo. |
| `hasNextPage` | done | T73 | Present; read through its own memo. |
| `hasPreviousPage` | done | T73 | Present; read through its own memo. |
| `isFetchNextPageError` | done | T73 | Present; read through its own memo. |
| `isFetchingNextPage` | done | T73 | Present; read through its own memo. |
| `isFetchPreviousPageError` | done | T73 | Present; read through its own memo. |
| `isFetchingPreviousPage` | done | T73 | Present; read through its own memo. |
| `pageParams` | done | T73 | Infinite data field, read after trimming with `maxPages`. |
| `data` | done | T14, T39 | Present; read through its own memo. |
| `variables` | done | T39 | Present; read through its own memo. |
| `error` | done | T39 | Present; read through its own memo. |
| `isError` | done | T39 | Present; read through its own memo. |
| `isIdle` | done | T39 | Present; read through its own memo. |
| `isPending` | done | T39 | Present; read through its own memo. |
| `isSuccess` | done | T39 | Present; read through its own memo. |
| `status` | done | T42 | Present; read through its own memo. |
| `mutate` | done | T41 | Never rejects: the adapter replaces the core's `mutate` with one that catches. |
| `mutateAsync` | done | T42 | Rejects with the error; used by the tests. |
| `reset` | done | T42 | Present; read through its own memo. |
| `context` | done | T41 | Set by `onMutate`; read in `onError` and `onSettled`. |
| `failureCount` | done | T39 | Present; read through its own memo. |
| `failureReason` | done | T39 | Present; read through its own memo. |
| `isPaused` | done | T39 | Present; read through its own memo. |
| `submittedAt` | done | T39 | Present; read through its own memo. |

## gpui-query: names and their TanStack equivalents

Every public name in `gpui-query`'s `hook` and `client` modules, with the TanStack name that does the same job. A name
that exists only because of Rust is `not applicable`, and says what it depends on.

| gpui-query name | TypeScript name | Status | Test | Note |
|---|---|---|---|---|
| `use_query` | useQuery | done | T56 | The query hook. |
| `use_query_with_policy` | useQuery with `staleTime` / `gcTime` / `retry` options | done | T57 | Policies are options. |
| `use_query_manual` | useQuery with `enabled: false`, then `refetch()` | done | T65 | Manual fetching is `enabled`. |
| `use_query_manual_opts` | useQuery with `enabled: false`, then `refetch()` | done | T65 | Same, with options. |
| `use_query_unsignalled` | useQuery with a plain options object | done | T27 | Read once, not tracked. |
| `use_query_unsignalled_opts` | useQuery with a plain options object | done | T27 | Same, with options. |
| `use_query_select` | useQuery with `select` | done | T66 | Derived data. |
| `fetch_query` | client.fetchQuery | done | T34 | Fetch and return. |
| `fetch_query_with_policy` | client.fetchQuery with `staleTime` | done | T12 | The policy is an option. |
| `fetch_query_with_signal` | client.fetchQuery with the `signal` in `queryFn` | done | T63 | The signal fires on cancel. |
| `use_infinite_query` | useInfiniteQuery | done | T73 | The infinite hook. |
| `fetch_next_page_infinite` | fetchNextPage | done | T73 | Method of the infinite result. |
| `fetch_previous_page_infinite` | fetchPreviousPage | done | T73 | Method of the infinite result. |
| `use_mutation` | useMutation | done | T41 | The mutation hook. |
| `mutate` | mutate / mutateAsync | done | T41 | `mutate` never rejects. |
| `mutate_with_callbacks` | mutate(variables, { onSuccess, onError, onSettled }) | done | T11 | Per-call callbacks. |
| `mutate_arc` | not applicable | not applicable |  | Shares a mutation across Rust owners with `Arc`. The result object is shared by reference in TypeScript, so there is no separate call. |
| `mutate_by_ref` | not applicable | not applicable |  | A Rust borrow of the mutation. TypeScript passes the same object; there is no borrow to choose. |
| `use_mutation_state` | useMutationState | done | T33 | The mutation-state hook. |
| `use_query_manual_opts` | useQuery with `enabled: false` | done | T65 | Same as `use_query_manual`. |
| `all_queries` | queryClient.getQueryCache().getAll() | done | T4 | Every query. |
| `all_infinite_queries` | queryClient.getQueryCache().getAll() | done | T4 | Infinite queries are queries in TanStack; filter by key or data shape. |
| `all_mutations` | queryClient.getMutationCache().getAll() | done | T4 | Every mutation. |
| `cancel_queries` | queryClient.cancelQueries | done | T63 | Cancels in-flight loads. |
| `collect_persist_snapshot` | persistQueryClientSave (dehydrates the client) | done | T53 | Snapshot of the persisted state. |
| `complete_failure` | query-core's fetch completion (internal) | done | T59 | Query-core completes every failed fetch; the retry tests drive it. |
| `complete_success` | query-core's fetch completion (internal) | done | T56 | Query-core completes every successful fetch. |
| `contains` | queryClient.getQueryCache().find | done | T4 | Whether a query exists. |
| `current_time_ms` | Date.now() on the NSH clock | done | T68 | Milliseconds since the epoch; the manual clock in tests. |
| `dehydrate` | dehydrate | done | T53 | Serialise the client. |
| `diagnostics` | useQueryDiagnostics | done | T17 | Snapshot of queries and mutations; see the difference on field names. |
| `empty` | new QueryClient() with empty caches | done | T4 | A fresh client has no queries. |
| `fetch_next_page_infinite (bucket)` | fetchNextPage | done | T73 | Same as above. |
| `force` | fetchQuery with `staleTime: 0` | done | T12 | Bypasses fresh data. |
| `gc` | gcTime (query-core collects unobserved entries) | done | T58 | Automatic collection; there is no manual call. |
| `gc_time` | gcTime option | done | T58 | The option. |
| `gc_with_time` | gcTime option | done | T58 | The option. |
| `get_query_data` | queryClient.getQueryData | done | T34 | Reads the cache. |
| `hydrate` | hydrate / HydrationBoundary | done | T31 | Applies a dehydrated state. |
| `infinite_query` | queryClient.infiniteQuery | done | T34 | Infinite query through the client. |
| `infinite_resource` | useInfiniteQuery | done | T73 | NSH's `resource()` is not used; the TanStack hook fills the role. |
| `infinite_resource_with_policies` | useInfiniteQuery with options | done | T73 | Policies are options. |
| `invalidate_queries` | queryClient.invalidateQueries | done | T62 | Refetches observed queries. |
| `keep_previous` | keepPreviousData | done | T66 | Placeholder with `isPlaceholderData`. |
| `matches` | matchQuery | done | T35 | Filter predicate. |
| `max_pages` | maxPages | done | T73 | Option. |
| `mutate (Rust fn)` | mutate | done | T41 | Same as above. |
| `new` | new QueryClient() | done | T56 | The constructor. |
| `next_request_id_for_infinite_key` | query-core's request dedupe per key (internal) | done | T56 | Two observers of one key share one fetch. |
| `next_request_id_for_key` | query-core's request dedupe per key (internal) | done | T56 | Same. |
| `observe` | QueryObserver.subscribe | done | T56 | Observing a query. |
| `on_error` | onError | done | T41 | Mutation callback. |
| `on_settled` | onSettled | done | T41 | Mutation callback. |
| `on_success` | onSuccess | done | T42 | Mutation callback. |
| `persist` | persistQueryClient | done | T53 | Saves the client. |
| `persist_with` | PersistQueryClientProvider persistOptions.persister | done | T2 | A persister given as an option. |
| `prepare_fetch_query` | client.fetchQuery | done | T34 | A prepared fetch is a call in TypeScript; difference: it starts when called, not later. |
| `prepare_prefetch_query` | client.prefetchQuery | done | T34 | Same. |
| `query` | queryClient.getQueryState | done | T34 | Read the query's state. |
| `register` | setQueryDefaults | done | T34 | Per-key defaults. Types need no registration in TypeScript. |
| `register_deserializer` | createStoragePersister deserialize | done | T51 | Custom format on read. |
| `register_mutation` | setMutationDefaults | done | T34 | Per-key mutation defaults. |
| `register_serializer` | createStoragePersister serialize | done | T51 | Custom format on write. |
| `remove_queries` | queryClient.removeQueries | done | T34 | Removes queries. |
| `request_policy` | networkMode | done | T20 | The request policy is `networkMode`. |
| `reset_queries` | queryClient.resetQueries | done | T34 | Resets queries. |
| `resource` | useQuery | done | T56 | NSH's `resource()` is not used; the TanStack hook fills the role. |
| `resource_with_policies` | useQuery with options | done | T57 | Policies are options. |
| `restore` | persistQueryClientRestore | done | T54 | Restores the client. |
| `retry_policy` | retry / retryDelay options | done | T59, T61 | Retry options. |
| `set_query_data` | queryClient.setQueryData | done | T34 | Writes the cache. |
| `unbounded_pages` | infinite query without maxPages | done | T6 | No limit is the default. |
| `use_infinite_query (Rust fn)` | useInfiniteQuery | done | T73 | Same as above. |
| `use_query (Rust fn)` | useQuery | done | T56 | Same as above. |
| `with_config` | new QueryClient({ defaultOptions }) | done | T5 | Client with defaults. |
| `with_gc_time` | defaultOptions.queries.gcTime | done | T5 | Default garbage-collection time. |
| `with_policies` | defaultOptions.queries.staleTime | done | T5 | Default staleness. |
| `with_query_data` | queryClient.setQueryData with an updater | done | T34 | Updater form. |
| `CacheMutation` | setQueryData with an updater | done | T34 | The in-place cache write, as an updater. |
| `ClientDiagnostic` | useQueryDiagnostics | done | T17 | Counts are `.length` of the lists. |
| `MutationDiagnostic` | useQueryDiagnostics mutations | done | T17 | Status and failure count. |
| `QueryDiagnostic` | useQueryDiagnostics queries | done | T17 | Key, status, fetch status, observers, staleness, last update, failures. |
| `DehydratedEntry` | dehydrate output | done | T53 | A query or mutation entry; its Rust type id has no TypeScript form. |
| `DehydratedState` | dehydrate output | done | T31 | The dehydrated client state. |
| `DeserializerRegistry` | createStoragePersister deserialize | done | T51 | Read-side codec. |
| `SerializerRegistry` | createStoragePersister serialize | done | T51 | Write-side codec. |
| `InfiniteQueryBucket` | query-core's infinite query (internal) | done | T73 | Infinite behaviour is query-core's. |
| `MutationBucket` | query-core's mutation cache (internal) | done | T41 | Mutation behaviour is query-core's. |
| `QueryBucket` | query-core's query cache (internal) | done | T56 | Query behaviour is query-core's. |
| `InfiniteQueryOptions` | useInfiniteQuery options | done | T73 | The options type. |
| `MutationCallbacks` | onSuccess / onError / onSettled | done | T41 | The callbacks type. |
| `MutationOptions` | useMutation options | done | T41 | The options type. |
| `QueryOptions` | useQuery options | done | T56 | The options type. |
| `NoopPersister` | createAsyncStoragePersister({ storage: null }) | done | T7 | Saves and restores nothing. |
| `Observer` | QueryObserver / MutationObserver | done | T56 | The observer classes. |
| `ObserverConfig` | QueryObserverOptions | done | T56 | The observer's options. |
| `PersistHandle` | persistQueryClient's [unsubscribe, promise] | done | T53 | The stop and the restore promise. |
| `PersistOptions` | PersistQueryClientProvider persistOptions | done | T45, T46, T47, T48 | The persistence options. |
| `PersistSnapshot` | persisted client shape | done | T53 | What is saved. |
| `PersistedEntry` | persisted query shape | done | T55 | One persisted query. |
| `PreparedFetch` | client.fetchQuery | done | T34 | Difference: TypeScript starts the fetch when called (see the prepare rows). |
| `QueryClient` | QueryClient | done | T56 | The client. |
| `PersistError` | retry strategy and onError | done | T3 | Failures reach `onError` (restore) or `retry` (save). |
| `PersistFilter` | dehydrateOptions.shouldDehydrateQuery | done | T47 | Which queries are stored. |
| `Persister (trait)` | Persister interface | done | T53 | Save, restore and remove. |
| `QueryPersister (trait)` | experimental_createQueryPersister | done | T55 | Per-query persister. |
| `ObservableResource (trait)` | QueryObserver.subscribe | done | T56 | Observing. |
| `QuerySelectResult (type)` | select result | done | T66 | The selected value. |
| `MutationErrorCallback, MutationSettledCallback, MutationSuccessCallback (types)` | mutation callback types | done | T41 | Callback types. |
| `PERSIST_VERSION` | buster | done | T45 | gpui's persisted format version is the TanStack `buster`: change it to discard stored data. |
| `current_time_ms (hook)` | Date.now() | done | T68 | Same as above. |

## Verification log

Run in `packages/nsh-query`, on the final code, with `nsh` 0.4.0 and `bun` 1.4.4.

| Gate | Result |
|---|---|
| `nsh check` | exit 0 (no output) |
| `nsh test` | `✓ 74 tests: 74 passed, 0 failed (4.2s)`, exit 0 |
| `nsh build` | `✓ built App.tsx into .nsh/out (271.8 KB js, 551.7 KB bytecode, 98ms)`; 0 warning lines |
| `nsh build --release` | `✓ built App.tsx into .nsh/out (83.4 KB js, 115.9 KB bytecode, 213ms)`; 0 warning lines. The release bundle is the test harness, with every screen |
| `grep -rn "new Map\|setTimeout\|retryDelay(" src` | no matches |
| `nsh test tests/smoke.test.tsx --target macos` | `✓ 1 test: 1 passed, 0 failed (913ms)`, exit 0 |
| Consumer project (linked package): `check` | exit 0 |
| Consumer project: `build` | `✓ built App.tsx into .nsh/out (144.2 KB js, 234.7 KB bytecode, 16ms)`; 0 warning lines |
| Consumer project: `test` | `✓ 1 test: 1 passed, 0 failed (24ms)` |
| Consumer project: `build --release` | `✓ built App.tsx into .nsh/out (32.7 KB js, 41.3 KB bytecode, 18ms)` |

Breaking the code on purpose, one group at a time (finish condition 9). Each break changed one spot, the named test failed,
and the file was restored to the checksum it had before the break:

| Group | Break | Result |
|---|---|---|
| Fine-grained `useQueries` | `src/useQueries.ts`: the field getter reads the result, not its memo | `✗ PersistQueryClientProvider's onError runs when restoring throws`; 64 tests, 63 passed |
| Error boundary | `src/useBaseQuery.ts`: the reset check removed | `✗ after a reset, a query that fails again ...`; 65 tests, 64 passed |
| Persister | `src/persist.ts`: the caller's `key` ignored | `✗ createStoragePersister key ...`; 65 tests, 64 passed |
| Diagnostics | `src/useQueryDiagnostics.ts`: the snapshot is always replaced | `✗ useQueryDiagnostics reports ...`; 65 tests, 64 passed |

A break of the persister throttle (forcing `throttleTime: 0`) hung `nsh test` for more than 15 minutes and was stopped; it is
not a recorded break. The `key` break was used instead.

Source checks at the end: no `BREAK`, `console.log` or `DBG` marker in `src`, `App.tsx` or `tests`.

Not verified: a visual check of any screen on macOS; the owner's `nsh-form` TypeScript fine-grained test (its package is not in this worktree).

