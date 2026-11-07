import { expect, test } from "nsh/test";
import * as nshQuery from "../src";

// Every value export of @tanstack/query-core's index.ts, which `export *` must carry over.
const coreValueExports = [
  "focusManager", "environmentManager", "dehydrate", "dehydrateQuery", "hydrate",
  "defaultShouldDehydrateMutation", "defaultShouldDehydrateQuery", "InfiniteQueryObserver",
  "MutationCache", "MutationObserver", "defaultScheduler", "notifyManager", "onlineManager",
  "QueriesObserver", "QueryCache", "QueryClient", "QueryObserver", "CancelledError",
  "isCancelledError", "timeoutManager", "hashKey", "isServer", "keepPreviousData",
  "matchMutation", "matchQuery", "noop", "partialMatchKey", "replaceEqualDeep",
  "shouldThrowError", "skipToken", "experimental_streamedQuery", "Mutation", "Query",
  "dataTagSymbol", "dataTagErrorSymbol", "unsetMarker",
];

// The names this package adds on top of the core.
const adapterExports = [
  "useQuery", "createQuery", "useInfiniteQuery", "createInfiniteQuery", "useMutation", "createMutation",
  "useQueries", "createQueries", "useIsFetching", "useIsMutating", "useMutationState",
  "queryOptions", "infiniteQueryOptions", "mutationOptions", "QueryClientProvider", "useQueryClient",
  "IsRestoringProvider", "useIsRestoring", "createStoragePersister", "PersistQueryClientProvider",
  "QueryErrorResetBoundary", "useQueryErrorResetBoundary", "usePrefetchQuery", "usePrefetchInfiniteQuery",
  "HydrationBoundary", "useSuspenseQuery", "useSuspenseInfiniteQuery", "useSuspenseQueries",
  "useQueryDiagnostics", "NshQueryError", "errorCodes", "createAsyncStoragePersister", "persistQueryClient",
  "persistQueryClientRestore", "persistQueryClientSave", "persistQueryClientSubscribe",
  "experimental_createQueryPersister", "removeOldestQuery", "PERSISTER_KEY_PREFIX",
];

test("every value export of query-core and every adapter name is reachable from the package", () => {
  const exported = nshQuery as Record<string, unknown>;
  for (const name of [...coreValueExports, ...adapterExports]) {
    expect(exported[name] === undefined).toBe(false);
  }
});
