# nsh-query

This is TanStack Query on Native Shell. It runs the unmodified `@tanstack/query-core` 5.107.1 under NSH's
QuickJS, and adds a thin adapter: reactive result objects, NSH's lifecycle for focus, NSH's batch for
notifications, and NSH's async `Storage` for persistence. The option names, result fields and behaviour are
TanStack's. For the API, read the TanStack Query documentation: <https://tanstack.com/query/latest>.

## Install

The package is not published yet. Put it in your app's `node_modules` by linking the folder, then install its
dependencies once:

```sh
mkdir -p node_modules
ln -s /path/to/nsh-query/packages/nsh-query node_modules/nsh-query
(cd /path/to/nsh-query/packages/nsh-query && bun install --ignore-scripts)
```

Import from `nsh-query`:

```tsx
import { createQuery, QueryClient, QueryClientProvider } from "nsh-query";
```

## A query

```tsx
import { mount, Text } from "nsh";
import { createQuery, QueryClient, QueryClientProvider } from "nsh-query";

const client = new QueryClient();

function Profile(props: { id: string }) {
  // The options are an accessor, so a changed `id` moves the query to its own cache entry.
  const user = createQuery<{ name: string }>(() => ({
    queryKey: ["user", props.id],
    queryFn: ({ signal }) => fetch(`https://api.example.com/users/${props.id}`, { signal }).then((r) => r.json()),
    staleTime: 60_000,
  }));

  return <Text>{user.isPending ? "Loading" : user.data?.name ?? "Failed"}</Text>;
}

mount(() => (
  <QueryClientProvider client={client}>
    <Profile id="1" />
  </QueryClientProvider>
));
```

Read result fields as properties, in JSX or in a closure. A binding on `user.data` runs again only when `data`
changes.

## A mutation

```tsx
import { useMutation } from "nsh-query";

function Rename(props: { id: string }) {
  const rename = useMutation<{ name: string }, Error, string>(() => ({
    mutationFn: (name) =>
      fetch(`https://api.example.com/users/${props.id}`, { method: "PATCH", body: JSON.stringify({ name }) }).then((r) => r.json()),
    onSuccess: () => client.invalidateQueries({ queryKey: ["user", props.id] }),
  }));

  return <Text role="button" onPress={() => rename.mutate("Ada")}>{rename.isPending ? "Saving" : "Rename"}</Text>;
}
```

`mutate` never rejects. Use `mutateAsync` when the caller needs the result or the error.

## An infinite list

```tsx
import { createInfiniteQuery } from "nsh-query";

type Page = { items: string[]; next: number | null };

function Feed() {
  const feed = createInfiniteQuery<Page, Error, { pages: Page[]; pageParams: number[] }, readonly unknown[], number>(() => ({
    queryKey: ["feed"],
    initialPageParam: 1,
    queryFn: ({ pageParam, signal }) =>
      fetch(`https://api.example.com/feed?page=${pageParam}`, { signal }).then((r) => r.json()),
    getNextPageParam: (last) => last.next ?? undefined,
    maxPages: 5,
  }));

  return (
    <Text role="button" onPress={() => feed.hasNextPage && feed.fetchNextPage()}>
      {feed.hasNextPage ? "More" : "End"}
    </Text>
  );
}
```

For a long list, render the items with NSH's `List`, which virtualizes rows.

## Persistence

`createStoragePersister` keeps the client in NSH's async `Storage`. Queries wait for the restore before they fetch.

```tsx
import { createStoragePersister, PersistQueryClientProvider, QueryClient } from "nsh-query";

const persistedClient = new QueryClient();

mount(() => (
  <PersistQueryClientProvider client={persistedClient} persistOptions={{ persister: createStoragePersister() }}>
    <Feed />
  </PersistQueryClientProvider>
));
```

`Storage` holds 5 MB, so a larger cache cannot be saved.

## What differs from TanStack Query on the web

- **Server detection.** NSH has no `window`, so the package tells query-core that it runs on a client
  (`src/nsh.ts`). Retries, garbage collection and refetch timers then behave as they do in a browser.
- **Focus** follows NSH's lifecycle. `refetchOnWindowFocus` means "visible again". `inactive` (visible without
  input) counts as focused and `background` does not, matching core's own visibility check.
- **Online** is always true unless the app calls `onlineManager.setOnline(false)`. NSH 0.4.0 reports no network
  status.
- **Suspense** is not available. NSH 0.4.0 has no Suspense boundary, so `suspense: true` and the
  `useSuspense*` hooks throw `NshQueryError` with the code `ERR_NSH_QUERY_NO_SUSPENSE`. Read `isPending` instead.
- **Errors the adapter raises** are `NshQueryError` objects with a stable `code`, such as
  `ERR_NSH_QUERY_NO_CLIENT`. Match on `code`, not on the message.
- **The client is read once per component.** Pass a different client to a new component.
- **`useQueries` without `combine`** returns an array-like value. Its items read the current result by index.
  With `combine`, reads follow the whole combined value.

The full list of names, their tests and what is not verified is in `docs/parity-typescript.md` in the repository.

## More names

- `usePrefetchQuery` and `usePrefetchInfiniteQuery` start a fetch while the component is shown.
- `HydrationBoundary` applies a dehydrated state before its children read it.
- `QueryErrorResetBoundary` and `useQueryErrorResetBoundary`: after a reset of an error boundary, the queries under
  it fetch again instead of throwing the old error again.
- `useQueryDiagnostics()` returns a snapshot of every query and mutation (key, status, fetch status, observers,
  staleness, last update). A binding wakes only when the snapshot changes, so it can drive a debug panel.
- Persistence: `createStoragePersister({ key, throttleTime, serialize, deserialize, retry })` is TanStack's
  async-storage persister over NSH `Storage`. `createAsyncStoragePersister` is re-exported for other storages, and
  every name of `@tanstack/query-persist-client-core` is re-exported, including `persistQueryClient`,
  `persistQueryClientRestore`, `persistQueryClientSave`, `persistQueryClientSubscribe` and
  `experimental_createQueryPersister`.

```tsx
import { createStoragePersister, useQueryDiagnostics } from "nsh-query";

const persister = createStoragePersister({ throttleTime: 1000 });

function Debug() {
  const diagnostics = useQueryDiagnostics();
  return <Text>{`${diagnostics().queries.length} queries`}</Text>;
}
```

## Tests

```sh
nsh check      # types
nsh build      # bundle; read every warning
nsh test       # 74 headless tests on a manual clock, with stubbed network
```
