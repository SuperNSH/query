import { expect, test } from "nsh/test";
import {
  createAsyncStoragePersister,
  environmentManager,
  isServer,
  NshQueryError,
  MutationObserver,
  QueryClient,
  removeOldestQuery,
  useQueryClient,
} from "../src";

test("cache helpers: empty, contains, all_queries, all_mutations and all_infinite_queries read the caches", async () => {
  const client = new QueryClient();
  expect(client.getQueryCache().getAll().length).toBe(0);
  client.setQueryData(["plain"], 1);
  await client.fetchInfiniteQuery({
    queryKey: ["infinite"],
    initialPageParam: 0,
    queryFn: ({ pageParam }) => pageParam,
    getNextPageParam: () => undefined,
  });
  await client.getMutationCache().build(client, { mutationFn: () => Promise.resolve(1) }).execute(undefined);
  expect(client.getQueryCache().find({ queryKey: ["plain"] }) !== undefined).toBe(true);
  expect(client.getQueryCache().find({ queryKey: ["missing"] }) === undefined).toBe(true);
  expect(client.getQueryCache().getAll().length).toBe(2);
  expect(client.getMutationCache().getAll().length).toBe(1);
  const infinite = client.getQueryCache().getAll().filter((query) => query.queryKey[0] === "infinite");
  expect(infinite.length).toBe(1);
});

test("with_config, with_gc_time and with_policies: a client keeps the default options it was made with", async () => {
  const client = new QueryClient({ defaultOptions: { queries: { gcTime: 1234, staleTime: 5678 } } });
  expect(client.getDefaultOptions().queries?.gcTime).toBe(1234);
  expect(client.getDefaultOptions().queries?.staleTime).toBe(5678);
  expect(client.defaultQueryOptions({ queryKey: ["x"] }).staleTime).toBe(5678);
});

test("unbounded_pages: an infinite query without maxPages keeps every page", async () => {
  const client = new QueryClient();
  const data = await client.fetchInfiniteQuery({
    queryKey: ["unbounded"],
    initialPageParam: 0,
    queryFn: ({ pageParam }) => pageParam,
    getNextPageParam: (page) => (page < 3 ? page + 1 : undefined),
    pages: 4,
  });
  expect(data.pages.length).toBe(4);
});

test("a persister over no storage (NoopPersister) saves nothing and restores nothing", async () => {
  const persister = createAsyncStoragePersister({ storage: null });
  await persister.persistClient({ timestamp: 1, buster: "", clientState: { mutations: [], queries: [] } });
  expect(await persister.restoreClient()).toBe(undefined);
  await persister.removeClient();
});

test("isServer: the exported constant reads true on NSH, while the core reads environmentManager, which says client", async () => {
  expect(environmentManager.isServer()).toBe(false);
  expect(isServer).toBe(true);
});

test("useQueryClient outside a provider throws NshQueryError with ERR_NSH_QUERY_NO_CLIENT", async () => {
  let code = "none";
  try {
    useQueryClient();
  } catch (error) {
    code = error instanceof NshQueryError ? error.code : String(error);
  }
  expect(code).toBe("ERR_NSH_QUERY_NO_CLIENT");
});

test("removeOldestQuery drops the query with the oldest data, and gives up when none are left", async () => {
  const query = (hash: string, dataUpdatedAt: number) => ({ queryKey: [hash], queryHash: hash, state: { dataUpdatedAt } });
  const persisted = {
    timestamp: 1,
    buster: "",
    clientState: { mutations: [], queries: [query("new", 20), query("old", 10)] },
  };
  const smaller = removeOldestQuery({ persistedClient: persisted as never, error: new Error("quota"), errorCount: 1 });
  expect(smaller?.clientState.queries.map((q) => q.queryHash)).toEqual(["new"]);
  const empty = removeOldestQuery({
    persistedClient: { timestamp: 1, buster: "", clientState: { mutations: [], queries: [] } } as never,
    error: new Error("quota"),
    errorCount: 1,
  });
  expect(empty).toBe(undefined);
});

test("mutate with per-call callbacks: onSuccess and onSettled given to mutate run after the mutation", async () => {
  const client = new QueryClient();
  const observer = new MutationObserver<number, Error, void>(client, { mutationFn: () => Promise.resolve(3) });
  const seen: string[] = [];
  // Per-call callbacks run only while the observer has a listener, as they do under useMutation.
  const stop = observer.subscribe(() => undefined);
  await observer.mutate(undefined, {
    onSuccess: (value) => {
      seen.push(`success ${value}`);
    },
    onSettled: () => {
      seen.push("settled");
    },
  });
  stop();
  expect(seen).toEqual(["success 3", "settled"]);
});

test("force: a fetch with staleTime 0 bypasses data that is still fresh", async () => {
  const client = new QueryClient();
  let calls = 0;
  const fresh = { queryKey: ["force"], queryFn: () => ++calls, staleTime: 60_000 };
  await client.fetchQuery(fresh);
  await client.fetchQuery(fresh);
  expect(calls).toBe(1);
  await client.fetchQuery({ ...fresh, staleTime: 0 });
  expect(calls).toBe(2);
});
