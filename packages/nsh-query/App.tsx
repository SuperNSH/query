import { cleanup, ErrorBoundary, type JSX, mount, type Signal, signal, Storage, Text, untracked, View } from "nsh";
import {
  createAsyncStoragePersister,
  dehydrate,
  experimental_createQueryPersister,
  experimental_streamedQuery,
  HydrationBoundary,
  infiniteQueryOptions,
  NshQueryError,
  persistQueryClient,
  persistQueryClientRestore,
  QueryErrorResetBoundary,
  QueryObserver,
  useQueryErrorResetBoundary,
  useQueryDiagnostics,
  usePrefetchInfiniteQuery,
  usePrefetchQuery,
  useSuspenseInfiniteQuery,
  useSuspenseQuery,
  useSuspenseQueries,
  mutationOptions,
  onlineManager,
  QueryClient,
  QueryClientProvider,
  createStoragePersister,
  createInfiniteQuery,
  createQuery,
  keepPreviousData,
  PersistQueryClientProvider,
  queryOptions,
  skipToken,
  useIsFetching,
  useIsMutating,
  useMutation,
  useMutationState,
  useQueries,
  useQueryClient,
} from "./src";

const API = "https://api.example.com";

/** Lines the screens write, which the tests read from the screen's texts. */
const logs = signal<string[]>([]);
const log = (line: string) => logs.set([...untracked(logs), line]);

/** The client of the visible screen. Cleared when the test switches screens. */
const client = new QueryClient();
const persistedClient = new QueryClient();

async function getJson<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(`${API}${path}`, { signal });
  if (!response.ok) throw new Error(`status ${response.status}`);
  return (await response.json()) as T;
}

async function postJson(path: string, body: unknown): Promise<unknown> {
  const response = await fetch(`${API}${path}`, { method: "POST", body: JSON.stringify(body) });
  if (!response.ok) throw new Error(`status ${response.status}`);
  return response.json();
}

// Test 1, 2, 3: one key, two observers, and a remount with a fresh or expired entry.
function SharedCount() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["shared"],
    queryFn: ({ signal }) => getJson("/shared", signal),
  }));
  return <Text>{`shared ${q.data?.n ?? "none"}`}</Text>;
}

function Count(props: { staleTime?: number; gcTime?: number }) {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["count"],
    queryFn: ({ signal }) => getJson("/count", signal),
    staleTime: props.staleTime,
    gcTime: props.gcTime,
  }));
  return <Text>{`count ${q.data?.n ?? "none"}`}</Text>;
}

function Cached(props: { staleTime?: number; gcTime?: number }) {
  const shown = signal(true);
  return (
    <View>
      <Text role="button" testID="toggle" onPress={() => shown.set(!shown())}>toggle</Text>
      <If when={shown()}>
        <Count staleTime={props.staleTime} gcTime={props.gcTime} />
      </If>
      <Text role="button" testID="invalidate" onPress={() => client.invalidateQueries({ queryKey: ["count"] })}>invalidate</Text>
    </View>
  );
}

// Test 4, 5, 6: retries, failures, and function-valued retry and retryDelay.
function Flaky(props: { retry: number | ((count: number) => boolean); retryDelay: number | ((count: number) => number) }) {
  const q = createQuery<{ ok: boolean }>(() => ({
    queryKey: ["flaky"],
    queryFn: ({ signal }) => getJson("/flaky", signal),
    retry: props.retry as never,
    retryDelay: props.retryDelay as never,
  }));
  return (
    <View>
      <Text>{`status ${q.status}`}</Text>
      <Text>{`failures ${q.failureCount}`}</Text>
      <Text>{`data ${q.data?.ok ?? "none"}`}</Text>
      <Text>{`reason ${q.failureReason?.message ?? "none"}`}</Text>
    </View>
  );
}

// Test 9: a tracked key moves between cache entries.
function ByKey() {
  const id = signal(1);
  const user = createQuery<{ name: string }>(() => ({
    queryKey: ["user", id()],
    queryFn: ({ queryKey }) => getJson(`/users/${String(queryKey[1])}`),
    staleTime: 60_000,
  }));
  return (
    <View>
      <Text>{`user ${user.data?.name ?? "none"}`}</Text>
      <Text role="button" testID="two" onPress={() => id.set(2)}>two</Text>
      <Text role="button" testID="one" onPress={() => id.set(1)}>one</Text>
    </View>
  );
}

// Test 10: enabled as a tracked value, and skipToken.
function Gated() {
  const on = signal(false);
  const gated = createQuery<{ v: string }>(() => ({
    queryKey: ["gated"],
    enabled: on(),
    queryFn: ({ signal }) => getJson("/gated", signal),
  }));
  const skipped = createQuery<{ v: string }>(() => ({
    queryKey: ["skipped"],
    queryFn: on() ? ({ signal }) => getJson("/skipped", signal) : skipToken,
  }));
  return (
    <View>
      <Text>{`gated ${gated.data?.v ?? "none"}`}</Text>
      <Text>{`gated fetch ${gated.fetchStatus}`}</Text>
      <Text>{`skipped ${skipped.data?.v ?? "none"}`}</Text>
      <Text role="button" testID="enable" onPress={() => on.set(true)}>enable</Text>
    </View>
  );
}

// Test 11: select and keepPreviousData, with isPlaceholderData.
function Pages() {
  const page = signal(1);
  const titled = createQuery<string>(() => ({
    queryKey: ["page", page()],
    queryFn: async ({ queryKey }) => (await getJson<{ title: string }>(`/page/${String(queryKey[1])}`)).title,
    select: (title: string) => title.toUpperCase(),
    placeholderData: keepPreviousData,
  }));
  return (
    <View>
      <Text>{`title ${titled.data ?? "none"}`}</Text>
      <Text>{`placeholder ${titled.isPlaceholderData}`}</Text>
      <Text role="button" testID="next" onPress={() => page.set(2)}>next</Text>
    </View>
  );
}

// Test 12: initialData, initialDataUpdatedAt and placeholderData.
function Seeded() {
  const seeded = createQuery<{ v: string }>(() => ({
    queryKey: ["seeded"],
    queryFn: ({ signal }) => getJson("/seeded", signal),
    initialData: { v: "seed" },
    staleTime: 60_000,
  }));
  const old = createQuery<{ v: string }>(() => ({
    queryKey: ["old"],
    queryFn: ({ signal }) => getJson("/old", signal),
    initialData: { v: "old" },
    initialDataUpdatedAt: Date.now() - 120_000,
    staleTime: 60_000,
  }));
  const slow = createQuery<{ v: string }>(() => ({
    queryKey: ["slow"],
    queryFn: ({ signal }) => getJson("/slow", signal),
    placeholderData: { v: "placeholder" },
  }));
  return (
    <View>
      <Text>{`seeded ${seeded.data?.v ?? "none"}`}</Text>
      <Text>{`old ${old.data?.v ?? "none"}`}</Text>
      <Text>{`slow ${slow.data?.v ?? "none"}`}</Text>
      <Text>{`slow placeholder ${slow.isPlaceholderData}`}</Text>
    </View>
  );
}

// Test 13: refetchInterval, with and without refetchIntervalInBackground.
function Polled() {
  const plain = createQuery<{ n: number }>(() => ({
    queryKey: ["poll"],
    queryFn: ({ signal }) => getJson("/poll", signal),
    refetchInterval: 1000,
  }));
  const background = createQuery<{ n: number }>(() => ({
    queryKey: ["pollbg"],
    queryFn: ({ signal }) => getJson("/pollbg", signal),
    refetchInterval: 1000,
    refetchIntervalInBackground: true,
  }));
  return (
    <View>
      <Text>{`poll ${plain.data?.n ?? "none"}`}</Text>
      <Text>{`pollbg ${background.data?.n ?? "none"}`}</Text>
    </View>
  );
}

// Test 14: refetchOnWindowFocus on return to the foreground.
function Focus() {
  const focused = createQuery<{ n: number }>(() => ({
    queryKey: ["focus"],
    queryFn: ({ signal }) => getJson("/focus", signal),
  }));
  const unfocused = createQuery<{ n: number }>(() => ({
    queryKey: ["nofocus"],
    queryFn: ({ signal }) => getJson("/nofocus", signal),
    refetchOnWindowFocus: false,
  }));
  return (
    <View>
      <Text>{`focus ${focused.data?.n ?? "none"}`}</Text>
      <Text>{`nofocus ${unfocused.data?.n ?? "none"}`}</Text>
    </View>
  );
}

// Test 15: onlineManager pauses fetches, and setting it back resumes them.
function Online() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["online"],
    queryFn: ({ signal }) => getJson("/online", signal),
  }));
  return (
    <View>
      <Text>{`online ${q.data?.n ?? "none"}`}</Text>
      <Text>{`fetch ${q.fetchStatus}`}</Text>
      <Text role="button" testID="offline" onPress={() => onlineManager.setOnline(false)}>offline</Text>
      <Text role="button" testID="online" onPress={() => onlineManager.setOnline(true)}>online again</Text>
      <Text role="button" testID="refetch" onPress={() => q.refetch()}>refetch</Text>
    </View>
  );
}

// Test 16: a binding on query.data runs again only when data changes.
const renders = signal(0);
const countRender = (value: string): string => {
  renders.set(untracked(renders) + 1);
  return value;
};

function Watch() {
  const q = createQuery<{ v: string }>(() => ({
    queryKey: ["watch"],
    queryFn: ({ signal }) => getJson("/watch", signal),
  }));
  return (
    <View>
      <Text>{countRender(q.data ? q.data.v : "none")}</Text>
      <Text>{`renders ${renders()}`}</Text>
      <Text>{`fetch ${q.fetchStatus}`}</Text>
      <Text role="button" testID="refetch" onPress={() => q.refetch()}>refetch</Text>
    </View>
  );
}

// Test 17: optimistic update, rollback, and the context reaching onError and onSettled.
function Optimistic() {
  const liked = createQuery<{ liked: boolean }>(() => ({
    queryKey: ["liked"],
    queryFn: ({ signal }) => getJson("/liked", signal),
  }));
  const like = useMutation<unknown, Error, boolean, { previous?: { liked: boolean } }>(() => ({
    mutationFn: (value: boolean) => postJson("/like", { liked: value }),
    onMutate: async (value) => {
      await client.cancelQueries({ queryKey: ["liked"] });
      const previous = client.getQueryData<{ liked: boolean }>(["liked"]);
      client.setQueryData(["liked"], { liked: value });
      return { previous };
    },
    onError: (_error, _value, context) => {
      client.setQueryData(["liked"], context?.previous);
      log(`rolled back to ${String(context?.previous?.liked)}`);
    },
    onSettled: (_data, _error, _value, context) => {
      log(`settled ${String(context?.previous?.liked)}`);
    },
  }));
  return (
    <View>
      <Text>{`liked ${liked.data?.liked ?? "none"}`}</Text>
      <Text>{`like error ${like.isError ? "yes" : "no"}`}</Text>
      <Text role="button" testID="like" onPress={() => like.mutate(true)}>like</Text>
    </View>
  );
}

// Test 18: invalidateQueries from onSuccess, mutateAsync result and rejection, and reset.
function Save() {
  const server = createQuery<{ n: number }>(() => ({
    queryKey: ["server"],
    queryFn: ({ signal }) => getJson("/server", signal),
  }));
  const save = useMutation<{ n: number }, Error, { n: number }>(() => ({
    mutationFn: (variables: { n: number }) => postJson("/server", variables) as Promise<{ n: number }>,
    onSuccess: () => client.invalidateQueries({ queryKey: ["server"] }),
  }));
  return (
    <View>
      <Text>{`server ${server.data?.n ?? "none"}`}</Text>
      <Text>{`status ${save.status}`}</Text>
      <Text role="button" testID="save" onPress={() => save.mutate({ n: 9 })}>save</Text>
      <Text role="button" testID="async" onPress={() => save.mutateAsync({ n: 7 }).then((r) => log(`async ${r.n}`))}>async</Text>
      <Text role="button" testID="reject" onPress={() => save.mutateAsync({ n: -1 }).catch((e: Error) => log(`rejected ${e.message}`))}>reject</Text>
      <Text role="button" testID="reset" onPress={() => save.reset()}>reset</Text>
    </View>
  );
}

// Test 19: mutation retry, and scope running calls in series.
function Retrying() {
  const flaky = useMutation<unknown, Error, void>(() => ({
    mutationFn: () => postJson("/flaky-save", {}),
    retry: 2,
    retryDelay: 1000,
  }));
  const first = useMutation<unknown, Error, void>(() => ({
    mutationFn: () => postJson("/serial/a", {}),
    scope: { id: "serial" },
  }));
  const second = useMutation<unknown, Error, void>(() => ({
    mutationFn: () => postJson("/serial/b", {}),
    scope: { id: "serial" },
  }));
  return (
    <View>
      <Text>{`flaky ${flaky.status} ${flaky.failureCount}`}</Text>
      <Text role="button" testID="flaky" onPress={() => flaky.mutate()}>flaky</Text>
      <Text role="button" testID="serial" onPress={() => { first.mutate(); second.mutate(); }}>serial</Text>
    </View>
  );
}

// Test 20: a mutation in flight finishes after the component that started it is gone.
function Mutator() {
  const save = useMutation<unknown, Error, void>(() => ({
    mutationFn: () => postJson("/leave", {}),
    onSuccess: () => client.setQueryData(["board"], { v: "saved" }),
  }));
  return <Text role="button" testID="leave-save" onPress={() => save.mutate()}>leave save</Text>;
}

function Leave() {
  const shown = signal(true);
  const board = createQuery<{ v: string }>(() => ({
    queryKey: ["board"],
    queryFn: () => ({ v: "initial" }),
    staleTime: Infinity,
  }));
  return (
    <View>
      <Text>{`board ${board.data?.v ?? "none"}`}</Text>
      <Text role="button" testID="hide" onPress={() => shown.set(!shown())}>hide</Text>
      <If when={shown()}>
        <Mutator />
      </If>
    </View>
  );
}

// Test 21: infinite queries, with maxPages, pageParams and refetch of every page.
type Page = { number: number; next: number | null; previous: number | null };

function Paged() {
  const pages = createInfiniteQuery<Page, Error, { pages: Page[]; pageParams: number[] }, readonly unknown[], number>(() => ({
    queryKey: ["pages"],
    initialPageParam: 2,
    queryFn: ({ pageParam, signal }) => getJson<Page>(`/pages/${pageParam}`, signal),
    getNextPageParam: (last) => last.next ?? undefined,
    getPreviousPageParam: (first) => first.previous ?? undefined,
    maxPages: 2,
  }));
  return (
    <View>
      <Text>{`pages ${pages.data?.pages.map((p) => p.number).join(",") ?? "none"}`}</Text>
      <Text>{`params ${pages.data?.pageParams.join(",") ?? "none"}`}</Text>
      <Text>{`hasNext ${pages.hasNextPage}`}</Text>
      <Text>{`hasPrev ${pages.hasPreviousPage}`}</Text>
      <Text role="button" testID="next" onPress={() => pages.fetchNextPage()}>next</Text>
      <Text role="button" testID="previous" onPress={() => pages.fetchPreviousPage()}>previous</Text>
      <Text role="button" testID="refetch" onPress={() => pages.refetch()}>refetch</Text>
    </View>
  );
}

// Test 22: useQueries with combine, useIsFetching, useIsMutating and useMutationState.
function Many() {
  const results = useQueries(() => ({
    queries: [
      { queryKey: ["a"], queryFn: ({ signal }: { signal: AbortSignal }) => getJson<{ n: number }>("/a", signal) },
      { queryKey: ["b"], queryFn: ({ signal }: { signal: AbortSignal }) => getJson<{ n: number }>("/b", signal) },
    ],
    combine: (rs) => {
      const items = rs as unknown as Array<{ data?: { n: number }; isPending: boolean }>;
      return {
        total: items.reduce((sum, r) => sum + (r.data?.n ?? 0), 0),
        pending: items.filter((r) => r.isPending).length,
      };
    },
  }));
  const fetching = useIsFetching();
  const mutating = useIsMutating();
  const mutation = useMutation<unknown, Error, string>(() => ({
    mutationFn: (note: string) => postJson("/note", { note }),
  }));
  const variables = useMutationState<string>(() => ({
    select: (m) => String(m.state.variables),
  }));
  return (
    <View>
      <Text>{`total ${results.total}`}</Text>
      <Text>{`pending ${results.pending}`}</Text>
      <Text>{`fetching ${fetching()}`}</Text>
      <Text>{`mutating ${mutating()}`}</Text>
      <Text>{`vars ${variables().join(",")}`}</Text>
      <Text role="button" testID="note-a" onPress={() => mutation.mutate("a")}>note a</Text>
      <Text role="button" testID="note-b" onPress={() => mutation.mutate("b")}>note b</Text>
    </View>
  );
}

// Test 23: the QueryClient methods, each logged to the screen.
function ClientActions() {
  const act = (testID: string, label: string, run: () => unknown) => (
    <Text role="button" testID={testID} onPress={() => void Promise.resolve(run()).then((v) => log(`${label} ${String(v)}`))}>{label}</Text>
  );
  const provided = useQueryClient();
  return (
    <View>
      <Text>{`same client ${provided === client}`}</Text>
      {act("set", "set", () => {
        client.setQueryData<number>(["num"], (old) => (old ?? 10) + 1);
        return client.getQueryData<number>(["num"]);
      })}
      {act("get", "get", () => client.getQueryData<number>(["num"]))}
      {act("state", "state", () => client.getQueryState(["num"])?.status)}
      {act("prefetch", "prefetch", async () => {
        await client.prefetchQuery({ queryKey: ["pre"], queryFn: () => ({ v: "prefetched" }) });
        return client.getQueryData<{ v: string }>(["pre"])?.v;
      })}
      {act("fetch", "fetch", () => client.fetchQuery({ queryKey: ["fq"], queryFn: () => "fetched" }))}
      {act("ensure", "ensure", () => client.ensureQueryData({ queryKey: ["num"], queryFn: () => 0 }))}
      {act("remove", "removed", () => {
        client.removeQueries({ queryKey: ["num"] });
        return client.getQueryData(["num"]) === undefined;
      })}
      {act("reset", "reset", () => {
        client.resetQueries({ queryKey: ["pre"] });
        return client.getQueryData(["pre"]) === undefined;
      })}
      <Text role="button" testID="start-slow" onPress={() => void client.prefetchQuery({ queryKey: ["slowc"], queryFn: ({ signal }) => getJson("/slowc", signal) })}>start slow</Text>
      {act("cancel", "cancelled", () => {
        client.cancelQueries({ queryKey: ["slowc"] });
        return client.getQueryState(["slowc"])?.fetchStatus;
      })}
      {act("defaults", "defaults", () => {
        client.setQueryDefaults(["d"], { staleTime: 5000 });
        return client.getQueryDefaults(["d", "x"]).staleTime;
      })}
      {act("mutation-defaults", "mutation defaults", () => {
        client.setMutationDefaults(["m"], { retry: 1 });
        return client.getMutationDefaults(["m"])?.retry;
      })}
      {act("default-options", "default options", () => {
        client.setDefaultOptions({ queries: { staleTime: 9000 } });
        return client.getDefaultOptions().queries?.staleTime;
      })}
      {act("set-many", "set many", () => {
        client.setQueryData<number>(["many", 1], 1);
        client.setQueriesData<number>({ queryKey: ["many"] }, (old) => (old ?? 0) + 5);
        return client.getQueryData<number>(["many", 1]);
      })}
      {act("get-many", "get many", () => client.getQueriesData<number>({ queryKey: ["many"] }).length)}
      {act("refetch-none", "refetch none", async () => (await client.refetchQueries({ queryKey: ["nothing"] })) === undefined)}
      {act("fetching", "fetching", () => client.isFetching())}
      {act("mutating", "mutating", () => client.isMutating())}
      {act("resume", "resumed", async () => {
        await client.resumePausedMutations();
        return "done";
      })}
      {act("infinite", "infinite", async () => {
        await client.prefetchInfiniteQuery({
          queryKey: ["inf"],
          initialPageParam: 1,
          queryFn: ({ pageParam }) => ({ page: pageParam }),
          getNextPageParam: () => undefined,
          pages: 1,
        });
        const data = await client.ensureInfiniteQueryData({
          queryKey: ["inf"],
          initialPageParam: 1,
          queryFn: () => ({ page: 99 }),
          getNextPageParam: () => undefined,
        });
        return data.pages.length;
      })}
      {act("infinite-query", "infinite query", async () => {
        const data = await client.infiniteQuery({
          queryKey: ["inf3"],
          initialPageParam: 5,
          queryFn: ({ pageParam }) => ({ page: pageParam }),
          getNextPageParam: () => undefined,
        });
        return data.pages[0]?.page;
      })}
      {act("fetch-infinite", "fetch infinite", async () => {
        const data = await client.fetchInfiniteQuery({
          queryKey: ["inf2"],
          initialPageParam: 3,
          queryFn: ({ pageParam }) => ({ page: pageParam }),
          getNextPageParam: () => undefined,
        });
        return data.pages[0]?.page;
      })}
    </View>
  );
}

// Test 24: query filters: exact, type, stale and predicate.
function Todo(props: { id: number }) {
  const q = createQuery<number>(() => ({
    queryKey: ["todos", props.id],
    queryFn: () => props.id,
    staleTime: 60_000,
  }));
  return <Text>{`todo ${q.data ?? "none"}`}</Text>;
}

function Filters() {
  client.setQueryData(["other"], "other");
  return (
    <View>
      <Todo id={1} />
      <Todo id={2} />
      <Text role="button" testID="run-filters" onPress={() => {
        const cache = client.getQueryCache();
        // An observed query is fresh; one with no observer is stale only once invalidated.
        client.invalidateQueries({ queryKey: ["other"] });
        log(`exact ${cache.findAll({ queryKey: ["todos"], exact: true }).length}`);
        log(`partial ${cache.findAll({ queryKey: ["todos"] }).length}`);
        log(`active ${cache.findAll({ type: "active" }).length}`);
        log(`inactive ${cache.findAll({ type: "inactive" }).length}`);
        log(`stale ${cache.findAll({ stale: true }).length}`);
        log(`fresh ${cache.findAll({ stale: false }).length}`);
        log(`predicate ${cache.findAll({ predicate: (q) => q.queryKey[0] === "other" }).length}`);
      }}>run filters</Text>
    </View>
  );
}

// Test 25: a capability refusal reaches query.error and is retried a bounded number of times.
function Denied() {
  const attempts = signal(0);
  const q = createQuery<unknown>(() => ({
    queryKey: ["denied"],
    queryFn: async ({ signal }) => {
      attempts.set(untracked(attempts) + 1);
      const response = await fetch("https://other.example.net/denied", { signal });
      return response.json();
    },
  }));
  return (
    <View>
      <Text>{`error ${(q.error as { code?: string } | null)?.code ?? "none"}`}</Text>
      <Text>{`attempts ${attempts()}`}</Text>
    </View>
  );
}

// Test 26: a client persisted to NSH's Storage is restored by a later app run.
function Saved() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["saved"],
    queryFn: ({ signal }) => getJson("/saved", signal),
    staleTime: 60_000,
  }));
  return <Text>{`saved ${q.data?.n ?? "none"}`}</Text>;
}

// Test 27: the smoke screen, with no options: it runs on a real window too.
function Smoke() {
  const q = createQuery<{ ok: boolean }>(() => ({
    queryKey: ["smoke"],
    queryFn: () => ({ ok: true }),
  }));
  return <Text>{`smoke ${q.data?.ok ? "yes" : "no"}`}</Text>;
}

const QUERY_FIELDS = [
  "data", "dataUpdatedAt", "error", "errorUpdatedAt", "failureCount", "failureReason", "errorUpdateCount",
  "isError", "isFetched", "isFetchedAfterMount", "isFetching", "isLoading", "isPending", "isLoadingError",
  "isInitialLoading", "isPaused", "isPlaceholderData", "isRefetchError", "isRefetching", "isStale", "isSuccess",
  "isEnabled", "refetch", "status", "fetchStatus",
];
const INFINITE_FIELDS = [
  "fetchNextPage", "fetchPreviousPage", "hasNextPage", "hasPreviousPage", "isFetchNextPageError",
  "isFetchingNextPage", "isFetchPreviousPageError", "isFetchingPreviousPage",
];
const MUTATION_FIELDS = [
  "data", "variables", "error", "isError", "isIdle", "isPending", "isSuccess", "status", "mutate", "mutateAsync",
  "reset", "context", "failureCount", "failureReason", "isPaused", "submittedAt",
];

// Test 31: every field the TanStack result types list is present on the result object.
function Fields() {
  const query = createQuery<string>(() => ({ queryKey: ["fields"], queryFn: () => "x" }));
  const infinite = createInfiniteQuery<string, Error, { pages: string[]; pageParams: number[] }, readonly unknown[], number>(() => ({
    queryKey: ["fields-inf"],
    initialPageParam: 0,
    queryFn: ({ pageParam }) => String(pageParam),
    getNextPageParam: () => undefined,
  }));
  const mutation = useMutation(() => ({ mutationFn: () => Promise.resolve(1) }));
  const verdict = (object: object, names: string[]): string => {
    const missing = names.filter((name) => !(name in object));
    return missing.length === 0 ? "ok" : `missing ${missing.join(" ")}`;
  };
  return (
    <View>
      <Text>{`query fields ${verdict(query, QUERY_FIELDS)}`}</Text>
      <Text>{`infinite fields ${verdict(infinite, [...QUERY_FIELDS, ...INFINITE_FIELDS])}`}</Text>
      <Text>{`mutation fields ${verdict(mutation, MUTATION_FIELDS)}`}</Text>
    </View>
  );
}

// Test 32: a query with an explicit client, and no provider around it.
function Explicit() {
  const q = createQuery<string>(
    () => ({ queryKey: ["explicit"], queryFn: () => "ok" }),
    () => client,
  );
  return <Text>{`explicit ${q.data ?? "none"}`}</Text>;
}

// Test 33: restoring a persisted client that throws reports through onError.
const failingPersister = {
  persistClient: () => undefined,
  restoreClient: () => {
    throw new Error("corrupt");
  },
  removeClient: () => undefined,
};

// Test 28: useQueries without combine reads each query by its index.
function Lists() {
  const queries = useQueries(() => ({
    queries: [
      { queryKey: ["l1"], queryFn: () => 1 },
      { queryKey: ["l2"], queryFn: () => 2 },
    ],
  }));
  return (
    <View>
      <Text>{`list ${queries.length} ${queries[0]?.data ?? "none"} ${queries[1]?.data ?? "none"}`}</Text>
      <Text>{`mapped ${queries.map((q) => q.data ?? "none").join(",")}`}</Text>
    </View>
  );
}

// Test 29: queryOptions, infiniteQueryOptions and mutationOptions return their options unchanged.
const sharedQuery = queryOptions({ queryKey: ["opts"], queryFn: () => 7 });
const sharedInfinite = infiniteQueryOptions({
  queryKey: ["opts-inf"],
  initialPageParam: 1,
  queryFn: ({ pageParam }: { pageParam: number }) => pageParam * 10,
  getNextPageParam: () => undefined,
});
const sharedMutation = mutationOptions({ mutationFn: (value: number) => Promise.resolve(value + 1) });

function Options() {
  const query = createQuery(() => sharedQuery);
  const infinite = createInfiniteQuery(() => sharedInfinite);
  const mutation = useMutation(() => sharedMutation);
  return (
    <View>
      <Text>{`options ${query.data ?? "none"}`}</Text>
      <Text>{`infinite options ${infinite.data?.pages[0] ?? "none"}`}</Text>
      <Text role="button" testID="mutate-options" onPress={() => mutation.mutateAsync(4).then((v) => log(`mutation options ${v}`))}>mutate</Text>
    </View>
  );
}

// Test 30: throwOnError hands the error to the nearest ErrorBoundary.
function Thrower() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["thrower"],
    queryFn: () => getJson("/thrower"),
    retry: false,
    throwOnError: true,
  }));
  return <Text>{`thrower ${q.data?.n ?? "none"}`}</Text>;
}

// ---- Gap screens (T35 onward) ------------------------------------------------------------

/** Counts how many times a binding ran, for the fine-grained tests. */
const countRun = <V,>(counter: Signal<number>, value: V): V => {
  counter.set(untracked(counter) + 1);
  return value;
};

const unmountClient = new QueryClient();

// unmount: a provider's client stops following focus when the provider goes.
function Unmount() {
  const shown = signal(true);
  const observer = new QueryObserver(unmountClient, {
    queryKey: ["unmount"],
    queryFn: ({ signal }) => getJson("/unmount", signal),
    staleTime: 0,
  });
  cleanup(observer.subscribe(() => undefined));
  return (
    <View>
      <Text role="button" testID="toggle" onPress={() => shown.set(!shown())}>toggle</Text>
      <If when={shown()}>
        <QueryClientProvider client={unmountClient}>
          <Text>mounted true</Text>
        </QueryClientProvider>
      </If>
    </View>
  );
}

// streamed: experimental_streamedQuery collects the chunks of an async iterable.
function Streamed() {
  const q = createQuery<number[]>(() => ({
    queryKey: ["streamed"],
    queryFn: experimental_streamedQuery<number>({
      streamFn: async function* () {
        yield 1;
        yield 2;
      },
    }),
  }));
  return <Text>{`stream ${q.data?.join(",") ?? "none"}`}</Text>;
}

// networkMode: "always" and "offlineFirst" run their first attempt offline; "online" waits.
function NetworkModes() {
  const always = createQuery<{ n: number }>(() => ({
    queryKey: ["always"],
    queryFn: ({ signal }) => getJson("/always", signal),
    networkMode: "always",
  }));
  const first = createQuery<{ n: number }>(() => ({
    queryKey: ["first"],
    queryFn: ({ signal }) => getJson("/first", signal),
    networkMode: "offlineFirst",
  }));
  const online = createQuery<{ n: number }>(() => ({
    queryKey: ["online-mode"],
    queryFn: ({ signal }) => getJson("/online-mode", signal),
    networkMode: "online",
  }));
  return (
    <View>
      <Text>{`always ${always.data?.n ?? "none"}`}</Text>
      <Text>{`first ${first.data?.n ?? "none"}`}</Text>
      <Text>{`online ${online.data?.n ?? "none"}`}</Text>
      <Text>{`online fetch ${online.fetchStatus}`}</Text>
      <Text role="button" testID="refetch-all" onPress={() => { always.refetch(); first.refetch(); online.refetch(); }}>refetch</Text>
      <Text role="button" testID="offline" onPress={() => onlineManager.setOnline(false)}>offline</Text>
      <Text role="button" testID="online" onPress={() => onlineManager.setOnline(true)}>online</Text>
    </View>
  );
}

// refetchOnMount: false keeps stale data on remount; the default refetches. The queries live in a
// child, so the toggle really unmounts and remounts their observers.
function MountedPair() {
  const kept = createQuery<{ n: number }>(() => ({
    queryKey: ["mount-no"],
    queryFn: ({ signal }) => getJson("/mount-no", signal),
    refetchOnMount: false,
  }));
  const refetched = createQuery<{ n: number }>(() => ({
    queryKey: ["mount-yes"],
    queryFn: ({ signal }) => getJson("/mount-yes", signal),
  }));
  return (
    <View>
      <Text>{`kept ${kept.data?.n ?? "none"}`}</Text>
      <Text>{`refetched ${refetched.data?.n ?? "none"}`}</Text>
    </View>
  );
}

function MountModes() {
  const shown = signal(true);
  return (
    <View>
      <Text role="button" testID="toggle" onPress={() => shown.set(!shown())}>toggle</Text>
      <If when={shown()}>
        <MountedPair />
      </If>
    </View>
  );
}

// refetchOnReconnect: a stale query refetches when the app goes back online, unless turned off.
function ReconnectModes() {
  const on = createQuery<{ n: number }>(() => ({
    queryKey: ["re-on"],
    queryFn: ({ signal }) => getJson("/re-on", signal),
    refetchOnReconnect: true,
  }));
  const off = createQuery<{ n: number }>(() => ({
    queryKey: ["re-off"],
    queryFn: ({ signal }) => getJson("/re-off", signal),
    refetchOnReconnect: false,
  }));
  return (
    <View>
      <Text>{`re-on ${on.data?.n ?? "none"}`}</Text>
      <Text>{`re-off ${off.data?.n ?? "none"}`}</Text>
      <Text role="button" testID="offline" onPress={() => onlineManager.setOnline(false)}>offline</Text>
      <Text role="button" testID="online" onPress={() => onlineManager.setOnline(true)}>online</Text>
    </View>
  );
}

// notifyOnChangeProps: only data changes notify, so a fetch that changes only fetchStatus is not shown.
function Notify() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["notify"],
    queryFn: ({ signal }) => getJson("/notify", signal),
    notifyOnChangeProps: ["data"],
  }));
  return (
    <View>
      <Text>{`notify ${q.data?.n ?? "none"}`}</Text>
      <Text>{`notify fetch ${q.fetchStatus}`}</Text>
      <Text role="button" testID="refetch" onPress={() => q.refetch()}>refetch</Text>
    </View>
  );
}

// meta: the query's meta reaches its queryFn.
function Meta() {
  const q = createQuery<string>(() => ({
    queryKey: ["meta"],
    meta: { label: "from meta" },
    queryFn: ({ meta }) => String((meta as { label?: string } | undefined)?.label ?? "none"),
  }));
  return <Text>{`meta ${q.data ?? "none"}`}</Text>;
}

// suspense: true throws a coded error to the nearest boundary.
function SuspenseOption() {
  createQuery<number>(() => ({ queryKey: ["suspense"], queryFn: () => 1, suspense: true }));
  return <Text>never</Text>;
}

function codeOf(error: unknown): string {
  return error instanceof NshQueryError ? error.code : String(error);
}

function SuspenseHookA() {
  void useSuspenseQuery();
  return <Text>never</Text>;
}
function SuspenseHookB() {
  void useSuspenseInfiniteQuery();
  return <Text>never</Text>;
}
function SuspenseHookC() {
  void useSuspenseQueries();
  return <Text>never</Text>;
}

// plain: options and the client argument need not be accessors.
function Plain() {
  const q = createQuery<string>({ queryKey: ["plain"], queryFn: () => "ok" });
  return <Text>{`plain ${q.data ?? "none"}`}</Text>;
}

const plainClient = new QueryClient();
function PlainClient() {
  const q = createQuery<number>(() => ({ queryKey: ["plain-client"], queryFn: () => 2 }), plainClient);
  return <Text>{`plain client ${q.data ?? "none"}`}</Text>;
}

// filters as accessors: the counts follow the filter that the signal holds.
function FilterAccessor() {
  const filter = signal({ queryKey: ["af-a"] as readonly unknown[] });
  const fetching = useIsFetching(() => filter());
  const mutating = useIsMutating(() => ({ mutationKey: [filter().queryKey[0]] }));
  createQuery(() => ({ queryKey: ["af-a"], queryFn: ({ signal }) => getJson("/af-a", signal) }));
  const mutation = useMutation<unknown, Error, void>(() => ({
    mutationKey: ["af-a"],
    mutationFn: () => postJson("/af-m", {}),
  }));
  return (
    <View>
      <Text>{`fetching ${fetching()}`}</Text>
      <Text>{`mutating ${mutating()}`}</Text>
      <Text role="button" testID="mutate" onPress={() => mutation.mutate()}>mutate</Text>
      <Text role="button" testID="switch" onPress={() => filter.set({ queryKey: ["af-b"] })}>switch</Text>
    </View>
  );
}

// reset: a thrown query error reaches the boundary; its reset retries the query.
function Explodes() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["reset"],
    queryFn: ({ signal }) => getJson("/reset", signal),
    retry: false,
    throwOnError: true,
  }));
  return (
    <View>
      <Text>{`reset ${q.data?.n ?? "none"}`}</Text>
      <Text>{`reset error ${q.isError}`}</Text>
    </View>
  );
}

function RetryButton(props: { reset: () => void }) {
  const boundary = useQueryErrorResetBoundary();
  return (
    <Text role="button" testID="retry" onPress={() => {
      boundary.reset();
      props.reset();
    }}>retry</Text>
  );
}

function ResetScreen() {
  return (
    <QueryErrorResetBoundary>
      <ErrorBoundary fallback={(error, reset) => (
        <View>
          <Text>{`reset failed ${(error as Error).message}`}</Text>
          <RetryButton reset={reset} />
        </View>
      )}>
        <Explodes />
      </ErrorBoundary>
    </QueryErrorResetBoundary>
  );
}

// prefetch: the hooks start the fetch, and a later useQuery reads it without a second request.
function Prefetch() {
  usePrefetchQuery(() => ({
    queryKey: ["pre-q"],
    queryFn: ({ signal }) => getJson("/pre", signal),
    staleTime: 60_000,
  }));
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["pre-q"],
    queryFn: ({ signal }) => getJson("/pre", signal),
    staleTime: 60_000,
  }));
  usePrefetchInfiniteQuery(() => ({
    queryKey: ["pre-inf"],
    initialPageParam: 1,
    queryFn: ({ pageParam }) => ({ page: pageParam }),
    getNextPageParam: () => undefined,
  }));
  const inf = createInfiniteQuery<{ page: number }, Error, { pages: Array<{ page: number }>; pageParams: number[] }, readonly unknown[], number>(() => ({
    queryKey: ["pre-inf"],
    initialPageParam: 1,
    queryFn: ({ pageParam }) => ({ page: pageParam }),
    getNextPageParam: () => undefined,
  }));
  return (
    <View>
      <Text>{`prefetched ${q.data?.n ?? "none"}`}</Text>
      <Text>{`prefetched inf ${inf.data?.pages.length ?? "none"}`}</Text>
    </View>
  );
}

// hydrate: a state dehydrated elsewhere is in the cache before the children run.
const hydrateSource = new QueryClient();
hydrateSource.setQueryData(["hydrated"], "from server");
const hydrateState = dehydrate(hydrateSource);

function Hydrated() {
  const q = createQuery<string>(() => ({
    queryKey: ["hydrated"],
    queryFn: ({ signal }) => getJson("/hydrated", signal),
    staleTime: 60_000,
  }));
  return <Text>{`hydrated ${q.data ?? "none"}`}</Text>;
}

// Persistence screens. Each run opens its own app, so the storage carries over only by the test.
const savedQuery = () =>
  createQuery<{ n: number }>(() => ({
    queryKey: ["saved"],
    queryFn: ({ signal }) => getJson("/saved", signal),
    staleTime: 60_000,
  }));

function SavedShown() {
  const q = savedQuery();
  return <Text>{`saved ${q.data?.n ?? "none"}`}</Text>;
}

let failures = 1;
const flakyStorage = {
  getItem: (key: string) => Storage.getItem(key),
  setItem: async (key: string, value: string) => {
    if (failures > 0) {
      failures -= 1;
      throw new Error("quota");
    }
    await Storage.setItem(key, value);
  },
  removeItem: (key: string) => Storage.removeItem(key),
};

function persistedScreen(persistOptions: Record<string, unknown>) {
  return (
    <PersistQueryClientProvider client={persistedClient} persistOptions={persistOptions as never}>
      <SavedShown />
    </PersistQueryClientProvider>
  );
}

// throttle: the first save is at once; a second within the interval waits for it.
function Throttled() {
  return (
    <View>
      <Text role="button" testID="one" onPress={() => persistedClient.setQueryData(["throttle"], "one")}>one</Text>
      <Text role="button" testID="two" onPress={() => persistedClient.setQueryData(["throttle"], "two")}>two</Text>
    </View>
  );
}

function SecretShown() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["secret"],
    queryFn: ({ signal }) => getJson("/secret", signal),
    staleTime: 60_000,
  }));
  return <Text>{`secret ${q.data?.n ?? "none"}`}</Text>;
}

// persistQueryClient and persistQueryClientRestore, called directly.
function PersistFn() {
  const [stop, restored] = persistQueryClient({
    queryClient: persistedClient,
    persister: createStoragePersister(),
  });
  cleanup(stop);
  restored.then(() => log("persist done"));
  return (
    <View>
      <Text role="button" testID="bump" onPress={() => persistedClient.setQueryData(["saved"], { n: 8 })}>bump</Text>
      <QueryClientProvider client={persistedClient}>
        <SavedShown />
      </QueryClientProvider>
    </View>
  );
}

function RestoreFn() {
  persistQueryClientRestore({ queryClient: persistedClient, persister: createStoragePersister() }).then(() =>
    log(`restored value ${JSON.stringify(persistedClient.getQueryData(["saved"]) ?? null)}`),
  );
  return <Text>restoring</Text>;
}

const queryPersister = experimental_createQueryPersister({ storage: Storage });
function QueryPersisted() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["qp"],
    queryFn: ({ signal }) => getJson("/qp", signal),
    staleTime: 60_000,
    persister: queryPersister.persisterFn,
  }));
  return <Text>{`qp ${q.data?.n ?? "none"}`}</Text>;
}

// Fine-grained: each binding counts its own runs.
const mutationDataRuns = signal(0);
const mutationStatusRuns = signal(0);
function FineMutation() {
  const m = useMutation<{ n: number }, Error, number>(() => ({
    mutationFn: (n: number) => postJson("/fine-m", { n }) as Promise<{ n: number }>,
  }));
  return (
    <View>
      <Text>{countRun(mutationDataRuns, `data ${m.data?.n ?? "none"}`)}</Text>
      <Text>{countRun(mutationStatusRuns, `status ${m.status}`)}</Text>
      <Text>{`data runs ${mutationDataRuns()}`}</Text>
      <Text>{`status runs ${mutationStatusRuns()}`}</Text>
      <Text role="button" testID="go" onPress={() => m.mutate(1)}>go</Text>
    </View>
  );
}

const infDataRuns = signal(0);
const infNextRuns = signal(0);
function FineInfinite() {
  const feed = createInfiniteQuery<{ number: number; next: number | null }, Error, { pages: Array<{ number: number }>; pageParams: number[] }, readonly unknown[], number>(() => ({
    queryKey: ["fine-inf"],
    initialPageParam: 1,
    queryFn: ({ pageParam, signal }) => getJson(`/fine-pages/${pageParam}`, signal),
    getNextPageParam: (last) => last.next ?? undefined,
  }));
  return (
    <View>
      <Text>{countRun(infDataRuns, `pages ${feed.data?.pages.map((p) => p.number).join(",") ?? "none"}`)}</Text>
      <Text>{countRun(infNextRuns, `more ${feed.hasNextPage}`)}</Text>
      <Text>{`data runs ${infDataRuns()}`}</Text>
      <Text>{`next runs ${infNextRuns()}`}</Text>
      <Text role="button" testID="refetch" onPress={() => feed.refetch()}>refetch</Text>
      <Text role="button" testID="next" onPress={() => feed.fetchNextPage()}>next</Text>
    </View>
  );
}

const aRuns = signal(0);
const bRuns = signal(0);
function FineQueries() {
  const extra = signal(false);
  const queries = useQueries(() => ({
    queries: [
      { queryKey: ["fa"], queryFn: () => "a" },
      { queryKey: ["fb"], queryFn: ({ signal }) => getJson<{ v: string }>("/fine-b", signal).then((r) => r.v) },
      ...(extra() ? [{ queryKey: ["fc"], queryFn: () => "c" }] : []),
    ],
  }));
  return (
    <View>
      <Text>{countRun(aRuns, `a ${String(queries[0]?.data ?? "none")}`)}</Text>
      <Text>{countRun(bRuns, `b ${String(queries[1]?.data ?? "none")}`)}</Text>
      <Text>{`a runs ${aRuns()}`}</Text>
      <Text>{`b runs ${bRuns()}`}</Text>
      <Text>{`length ${queries.length}`}</Text>
      <Text>{`c ${String(queries[2]?.data ?? "none")}`}</Text>
      <Text role="button" testID="refetch-b" onPress={() => queries[1]?.refetch()}>refetch b</Text>
      <Text role="button" testID="grow" onPress={() => extra.set(true)}>grow</Text>
    </View>
  );
}

// Diagnostics.
const diagnosticRuns = signal(0);
function Diagnostics() {
  const diagnostics = useQueryDiagnostics();
  createQuery(() => ({
    queryKey: ["d-a"],
    queryFn: ({ signal }) => getJson("/d-a", signal),
    staleTime: 60_000,
  }));
  const mutation = useMutation<unknown, Error, void>(() => ({
    mutationKey: ["d-m"],
    mutationFn: () => postJson("/d-m", {}),
  }));
  return (
    <View>
      <Text>{countRun(diagnosticRuns, `diagnostics ${diagnostics().queries.length}`)}</Text>
      <Text>{`diagnostic runs ${diagnosticRuns()}`}</Text>
      {diagnostics().queries.map((entry) => (
        <Text>{`query ${entry.queryKey.join("/")} ${entry.status} ${entry.fetchStatus} observers ${entry.observers} stale ${entry.stale}`}</Text>
      ))}
      <Text>{`mutations ${diagnostics().mutations.map((m) => m.status).join(",") || "none"}`}</Text>
      <Text role="button" testID="mutate" onPress={() => mutation.mutate()}>mutate</Text>
      <Text role="button" testID="same" onPress={() => {
        // A cache event that changes no diagnostic: the query's observers were notified, nothing else moved.
        const query = client.getQueryCache().find({ queryKey: ["d-a"] });
        if (query) client.getQueryCache().notify({ type: "observerResultsUpdated", query });
      }}>same</Text>
    </View>
  );
}

const screens: Record<string, () => JSX.Element> = {
  smoke: () => <QueryClientProvider client={client}><Smoke /></QueryClientProvider>,
  shared: () => (
    <QueryClientProvider client={client}>
      <SharedCount />
      <SharedCount />
    </QueryClientProvider>
  ),
  cached: () => <QueryClientProvider client={client}><Cached staleTime={60_000} /></QueryClientProvider>,
  gc: () => <QueryClientProvider client={client}><Cached gcTime={1000} /></QueryClientProvider>,
  retry: () => <QueryClientProvider client={client}><Flaky retry={2} retryDelay={1000} /></QueryClientProvider>,
  fails: () => <QueryClientProvider client={client}><Flaky retry={2} retryDelay={1000} /></QueryClientProvider>,
  retryfns: () => (
    <QueryClientProvider client={client}>
      <Flaky retry={(count) => count < 2} retryDelay={(count) => (count + 1) * 100} />
    </QueryClientProvider>
  ),
  bykey: () => <QueryClientProvider client={client}><ByKey /></QueryClientProvider>,
  gated: () => <QueryClientProvider client={client}><Gated /></QueryClientProvider>,
  select: () => <QueryClientProvider client={client}><Pages /></QueryClientProvider>,
  seeded: () => <QueryClientProvider client={client}><Seeded /></QueryClientProvider>,
  interval: () => <QueryClientProvider client={client}><Polled /></QueryClientProvider>,
  focus: () => <QueryClientProvider client={client}><Focus /></QueryClientProvider>,
  online: () => <QueryClientProvider client={client}><Online /></QueryClientProvider>,
  renders: () => <QueryClientProvider client={client}><Watch /></QueryClientProvider>,
  optimistic: () => <QueryClientProvider client={client}><Optimistic /></QueryClientProvider>,
  invalidate: () => <QueryClientProvider client={client}><Invalidating /></QueryClientProvider>,
  abort: () => <QueryClientProvider client={client}><Abort /></QueryClientProvider>,
  save: () => <QueryClientProvider client={client}><Save /></QueryClientProvider>,
  retrying: () => <QueryClientProvider client={client}><Retrying /></QueryClientProvider>,
  leave: () => <QueryClientProvider client={client}><Leave /></QueryClientProvider>,
  pages: () => <QueryClientProvider client={client}><Paged /></QueryClientProvider>,
  many: () => <QueryClientProvider client={client}><Many /></QueryClientProvider>,
  client: () => <QueryClientProvider client={client}><ClientActions /></QueryClientProvider>,
  filters: () => <QueryClientProvider client={client}><Filters /></QueryClientProvider>,
  denied: () => <QueryClientProvider client={client}><Denied /></QueryClientProvider>,
  persist: () => (
    <PersistQueryClientProvider
      client={persistedClient}
      persistOptions={{ persister: createStoragePersister() }}
      onSuccess={() => log("restored")}
      onError={() => log("restore failed")}
    >
      <Saved />
    </PersistQueryClientProvider>
  ),
  lists: () => <QueryClientProvider client={client}><Lists /></QueryClientProvider>,
  explicit: () => <Explicit />,
  "persist-fail": () => (
    <PersistQueryClientProvider
      client={persistedClient}
      persistOptions={{ persister: failingPersister }}
      onError={() => log("restore failed")}
    >
      <Saved />
    </PersistQueryClientProvider>
  ),
  fields: () => <QueryClientProvider client={client}><Fields /></QueryClientProvider>,
  options: () => <QueryClientProvider client={client}><Options /></QueryClientProvider>,
  boundary: () => (
    <QueryClientProvider client={client}>
      <ErrorBoundary fallback={(error: unknown) => <Text>{`boundary ${(error as Error).message}`}</Text>}>
        <Thrower />
      </ErrorBoundary>
    </QueryClientProvider>
  ),
  unmount: () => <Unmount />,
  streamed: () => <QueryClientProvider client={client}><Streamed /></QueryClientProvider>,
  network: () => <QueryClientProvider client={client}><NetworkModes /></QueryClientProvider>,
  mount: () => <QueryClientProvider client={client}><MountModes /></QueryClientProvider>,
  reconnect: () => <QueryClientProvider client={client}><ReconnectModes /></QueryClientProvider>,
  notify: () => <QueryClientProvider client={client}><Notify /></QueryClientProvider>,
  meta: () => <QueryClientProvider client={client}><Meta /></QueryClientProvider>,
  suspense: () => (
    <QueryClientProvider client={client}>
      <ErrorBoundary fallback={(error: unknown) => <Text>{`suspense ${codeOf(error)}`}</Text>}>
        <SuspenseOption />
      </ErrorBoundary>
    </QueryClientProvider>
  ),
  "suspense-hooks": () => (
    <QueryClientProvider client={client}>
      <View>
        <ErrorBoundary fallback={(error: unknown) => <Text>{`hook a ${codeOf(error)}`}</Text>}><SuspenseHookA /></ErrorBoundary>
        <ErrorBoundary fallback={(error: unknown) => <Text>{`hook b ${codeOf(error)}`}</Text>}><SuspenseHookB /></ErrorBoundary>
        <ErrorBoundary fallback={(error: unknown) => <Text>{`hook c ${codeOf(error)}`}</Text>}><SuspenseHookC /></ErrorBoundary>
      </View>
    </QueryClientProvider>
  ),
  plain: () => <QueryClientProvider client={client}><Plain /></QueryClientProvider>,
  "plain-client": () => <PlainClient />,
  "filter-accessor": () => <QueryClientProvider client={client}><FilterAccessor /></QueryClientProvider>,
  reset: () => <QueryClientProvider client={client}><ResetScreen /></QueryClientProvider>,
  prefetch: () => <QueryClientProvider client={client}><Prefetch /></QueryClientProvider>,
  hydrate: () => (
    <QueryClientProvider client={client}>
      <HydrationBoundary state={hydrateState}>
        <Hydrated />
      </HydrationBoundary>
    </QueryClientProvider>
  ),
  "persist-buster": () => persistedScreen({ persister: createStoragePersister(), buster: "new" }),
  "persist-maxage": () => persistedScreen({ persister: createStoragePersister(), maxAge: 1000 }),
  "persist-filter": () => (
    <PersistQueryClientProvider client={persistedClient} persistOptions={{ persister: createStoragePersister(), dehydrateOptions: { shouldDehydrateQuery: (query) => query.queryKey[0] !== "secret" } }}>
      <View>
        <SavedShown />
        <SecretShown />
      </View>
    </PersistQueryClientProvider>
  ),
  "persist-hydrate": () =>
    persistedScreen({
      persister: createStoragePersister(),
      hydrateOptions: { defaultOptions: { deserializeData: (data: unknown) => ({ n: (data as { n: number }).n * 100 }) } },
    }),
  "persist-key": () => persistedScreen({ persister: createStoragePersister({ key: "custom-key" }) }),
  "persist-throttle": () => (
    <PersistQueryClientProvider client={persistedClient} persistOptions={{ persister: createStoragePersister({ throttleTime: 5000 }) }}>
      <Throttled />
    </PersistQueryClientProvider>
  ),
  "persist-codec": () =>
    persistedScreen({
      persister: createStoragePersister({
        serialize: (persisted) => `v2:${JSON.stringify(persisted)}`,
        deserialize: (stored: string) => JSON.parse(stored.slice(3)),
      }),
    }),
  "persist-retry": () =>
    persistedScreen({
      persister: createAsyncStoragePersister({
        storage: flakyStorage,
        key: "nsh-query-client",
        throttleTime: 0,
        retry: ({ persistedClient: pending, errorCount }) => {
          log(`retry ${errorCount}`);
          return errorCount < 2 ? pending : undefined;
        },
      }),
    }),
  "persist-fn": () => <PersistFn />,
  "restore-fn": () => <RestoreFn />,
  "query-persister": () => <QueryClientProvider client={client}><QueryPersisted /></QueryClientProvider>,
  "fine-mutation": () => <QueryClientProvider client={client}><FineMutation /></QueryClientProvider>,
  "fine-infinite": () => <QueryClientProvider client={client}><FineInfinite /></QueryClientProvider>,
  "fine-queries": () => <QueryClientProvider client={client}><FineQueries /></QueryClientProvider>,
  diagnostics: () => <QueryClientProvider client={client}><Diagnostics /></QueryClientProvider>,
};

// Test 7: invalidation replaces a refetch in flight, once the query has data (query-core cancels
// a fetch only when there is data to keep).
function Invalidating() {
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["count"],
    queryFn: ({ signal }) => getJson("/count", signal),
    staleTime: 0,
  }));
  return (
    <View>
      <Text>{`count ${q.data?.n ?? "none"}`}</Text>
      <Text role="button" testID="slow" onPress={() => client.refetchQueries({ queryKey: ["count"] })}>slow</Text>
      <Text role="button" testID="invalidate" onPress={() => client.invalidateQueries({ queryKey: ["count"] })}>invalidate</Text>
    </View>
  );
}

// Test 8: the signal passed to fetch fires when the query is cancelled.
function Abort() {
  const aborted = signal(false);
  const q = createQuery<{ n: number }>(() => ({
    queryKey: ["abort"],
    queryFn: ({ signal }) => {
      signal.addEventListener("abort", () => aborted.set(true));
      return getJson("/abort", signal);
    },
  }));
  return (
    <View>
      <Text>{`abort ${q.data?.n ?? "none"}`}</Text>
      <Text>{`aborted ${aborted()}`}</Text>
      <Text role="button" testID="cancel" onPress={() => client.cancelQueries({ queryKey: ["abort"] })}>cancel</Text>
    </View>
  );
}

const shownScreen = signal("none");

function screenFor(name: string): JSX.Element | null {
  const render = screens[name];
  return render ? render() : null;
}

function App() {
  const show = (name: string) => () => {
    client.clear();
    persistedClient.clear();
    logs.set([]);
    shownScreen.set(name);
  };
  return (
    <View>
      {shownScreen() === "none" ? (
        <View>
          {Object.keys(screens).map((name) => (
            <Text role="button" testID={`open-${name}`} onPress={show(name)}>{name}</Text>
          ))}
        </View>
      ) : null}
      <View>{logs().map((line) => <Text>{line}</Text>)}</View>
      <View>{screenFor(shownScreen())}</View>
    </View>
  );
}

mount(App);
