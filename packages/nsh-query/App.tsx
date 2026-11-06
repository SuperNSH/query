import { type JSX, mount, signal, Text, View } from "nsh";
import {
  createInfiniteQuery,
  createMutation,
  createQuery,
  noRetries,
  queryClient,
  setQueryData,
} from "./src";

const API = "https://api.example.com";

async function getJson<T>(url: string, signal: AbortSignal): Promise<T> {
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`status ${response.status}`);
  return response.json() as Promise<T>;
}

function Row(props: { id: string; label: string; onPress?: () => void }) {
  return (
    <Text testID={props.id} role={props.onPress ? "button" : undefined} onPress={props.onPress}>
      {props.label}
    </Text>
  );
}

function Counter() {
  const count = createQuery<{ n: number }>({
    key: "count",
    cache: { ttlMs: 60_000 },
    retry: noRetries,
    fetch: ({ signal }) => getJson(`${API}/count`, signal),
  });
  return <Row id="count" label={`count ${count.data()?.n ?? "none"}`} />;
}

function Cached() {
  const shown = signal(true);
  return (
    <View>
      <Row id="toggle" label="toggle" onPress={() => shown.set(!shown())} />
      <If when={shown()}>
        <Counter />
        <Counter />
      </If>
      <Row id="invalidate" label="invalidate" onPress={() => queryClient.invalidate("count")} />
    </View>
  );
}

function Flaky() {
  const query = createQuery<{ ok: boolean }>({
    key: "flaky",
    retry: { maxRetries: 2, delayMs: 1000 },
    fetch: ({ signal }) => getJson(`${API}/flaky`, signal),
  });
  return (
    <View>
      <Row id="flaky" label={`flaky ${query.data()?.ok ?? "none"}`} />
      <Row id="failed" label={`failed ${query.isFailure()}`} />
      <Row id="retries" label={`retries ${query.retryCount()}`} />
      <Row id="error" label={`error ${query.error() ? String(query.error()) : "none"}`} />
    </View>
  );
}

function ByKey() {
  const id = signal(1);
  const user = createQuery<{ name: string }>({
    key: () => ["users", id()],
    fetch: ({ key, signal }) => getJson(`${API}/users/${key[1]}`, signal),
  });
  return (
    <View>
      <Row id="user" label={`user ${user.data()?.name ?? "none"}`} />
      <Row id="two" label="two" onPress={() => id.set(2)} />
      <Row id="one" label="one" onPress={() => id.set(1)} />
    </View>
  );
}

function Gated() {
  const on = signal(false);
  const query = createQuery<{ v: string }>({
    key: "gated",
    enabled: () => on(),
    fetch: ({ signal }) => getJson(`${API}/gated`, signal),
  });
  return (
    <View>
      <Row id="gated" label={`gated ${query.data()?.v ?? "idle"}`} />
      <Row id="enable" label="enable" onPress={() => on.set(true)} />
    </View>
  );
}

function Seeded() {
  setQueryData("seeded", { v: "from cache" });
  const query = createQuery<{ v: string }>({
    key: "seeded",
    fetch: ({ signal }) => getJson(`${API}/seeded`, signal),
  });
  return <Row id="seeded" label={`seeded ${query.data()?.v ?? "none"}`} />;
}

function Save() {
  const count = createQuery<{ n: number }>({
    key: "server",
    retry: noRetries,
    fetch: ({ signal }) => getJson(`${API}/server`, signal),
  });
  const save = createMutation<number, { n: number }>({
    invalidates: ["server"],
    mutate: async (n, { signal }) => {
      const response = await fetch(`${API}/server`, { method: "POST", body: JSON.stringify({ n }), signal });
      return response.json() as Promise<{ n: number }>;
    },
  });
  return (
    <View>
      <Row id="server" label={`server ${count.data()?.n ?? "none"}`} />
      <Row id="saving" label={`saving ${save.isLoading()}`} />
      <Row id="save" label="save" onPress={() => save.mutate(9)} />
    </View>
  );
}

function Optimistic() {
  const liked = createQuery<boolean>({ key: "liked", fetch: async () => false });
  const like = createMutation<boolean, boolean>({
    retry: { maxRetries: 1, delayMs: 100 },
    mutate: async () => {
      throw new Error("refused");
    },
    onMutate: (next) => liked.setData(next),
    onError: () => liked.rollback(),
  });
  return (
    <View>
      <Row id="liked" label={`liked ${liked.data() ?? "none"}`} />
      <Row id="like-error" label={`like error ${like.error() ? "yes" : "no"}`} />
      <Row id="like" label="like" onPress={() => like.mutate(true)} />
    </View>
  );
}

type Page = { number: number; next: number | null; previous: number | null };

function Pages() {
  const pages = createInfiniteQuery<number, Page>({
    key: "pages",
    initialPageParam: 2,
    retry: noRetries,
    getNextPageParam: (page) => page.next,
    getPreviousPageParam: (page) => page.previous,
    fetchPage: ({ pageParam, signal }) => getJson(`${API}/pages/${pageParam}`, signal),
  });
  return (
    <View>
      <Row id="pages" label={`pages ${pages.pages().map((p) => p.number).join(",") || "none"}`} />
      <Row id="more" label={`more ${pages.hasNextPage()}`} />
      <Row id="earlier" label={`earlier ${pages.hasPreviousPage()}`} />
      <Row id="next" label="next" onPress={() => pages.fetchNextPage()} />
      <Row id="previous" label="previous" onPress={() => pages.fetchPreviousPage()} />
      <Row id="refetch" label="refetch" onPress={() => pages.refetch()} />
    </View>
  );
}

const screens: Record<string, () => JSX.Element> = {
  Cached,
  Flaky,
  ByKey,
  Gated,
  Seeded,
  Save,
  Optimistic,
  Pages,
};

function App() {
  const screen = signal<string | null>(null);
  return (
    <View class="flex-1 gap-2 p-4">
      <If when={screen()} else={<Each of={Object.keys(screens)}>{(name) => <Row id={`open-${name()}`} label={`open ${name()}`} onPress={() => screen.set(name())} />}</Each>}>
        {(name) => screens[name()]()}
      </If>
    </View>
  );
}

mount(App);
