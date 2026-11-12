# nsh-query

Async state for [Native Shell](https://nsh.dev) apps, based on [TanStack Query](https://tanstack.com/query/latest) v5: caching, deduplication, retries, invalidation, mutations and infinite lists. If you know TanStack Query, you already know the names and how they behave, so its documentation is the main reference.

There are two versions:

| Version | Path | What it is | For |
|---|---|---|---|
| TypeScript package | `packages/nsh-query` | `@tanstack/query-core` 5.107.1, unchanged, with a thin NSH adapter | TSX apps, signed packages, anything with no Rust crate |
| Rust crate | `crates/nsh-query` | A hand-written port of TanStack Query's behavior, in snake_case | Rust and Rust-with-TSX apps |

## Why two versions

TSX runs on NSH's prebuilt runtime in QuickJS, and TanStack's JavaScript core runs there unmodified. The TypeScript package adds only what NSH needs: result objects that read as signals, NSH's lifecycle for focus, its batching for notifications, and its async `Storage` for persistence.

A Rust app cannot run that core, so the crate ports its behavior. It builds on [`gpui-query`](https://github.com/freeoxide/gpui-query)'s `core` feature, a state machine with no GPUI dependency (`QueryResource`, `MutationResource`, `InfiniteQueryResource`, `CachePolicy`, `RetryPolicy`, `QueryKey`), and replaces its GPUI layers with NSH signals, tasks and timers.

Requires NSH 0.5.0.

## TypeScript

```sh
mkdir -p node_modules && ln -s /path/to/nsh-query/packages/nsh-query node_modules/nsh-query
(cd /path/to/nsh-query/packages/nsh-query && bun install --ignore-scripts)
```

```tsx
import { mount, Text, View } from "nsh";
import { createMutation, createQuery, QueryClient, QueryClientProvider, useQueryClient } from "nsh-query";

const client = new QueryClient();

function Todos() {
  const queryClient = useQueryClient();
  const todos = createQuery<string[]>(() => ({
    queryKey: ["todos"],
    queryFn: ({ signal }) => fetch("https://api.example.com/todos", { signal }).then((r) => r.json()),
    staleTime: 30_000,
  }));
  const add = createMutation<void, Error, string>(() => ({
    mutationFn: async (title) => {
      await fetch("https://api.example.com/todos", { method: "POST", body: JSON.stringify({ title }) });
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["todos"] }),
  }));
  return (
    <View>
      <Text>{`${todos.data?.length ?? 0} todos`}</Text>
      <Text role="button" onPress={() => add.mutate("Walk")}>
        Add
      </Text>
    </View>
  );
}

mount(() => (
  <QueryClientProvider client={client}>
    <Todos />
  </QueryClientProvider>
));
```

[`packages/nsh-query/README.md`](packages/nsh-query/README.md) covers infinite lists and persistence.

## Rust

```toml
[dependencies]
nsh-query = { path = "../nsh-query/crates/nsh-query" }
```

```rust
use std::time::Duration;

use nsh_core::prelude::*;
use nsh_query::{MutationOptions, QueryOptions, use_mutation_with, use_query_with};

#[component]
pub fn Todos() -> impl IntoElement {
    let todos = use_query_with::<Vec<String>, String, _, _>(
        "todos",
        QueryOptions::new().stale_time(Duration::from_secs(30)),
        |_| async { api::todos().await },
    );
    let add = use_mutation_with::<String, (), String, _, _>(
        MutationOptions::new().invalidates("todos"),
        |title, _| async move { api::add(title).await },
    );
    view()
        .child(text(move || format!("{} todos", todos.with(|t| t.map_or(0, Vec::len)))))
        .child(text("Add").on_press(move || add.mutate("Walk".to_owned())))
}
```

## The same concepts in both

| TanStack Query | Rust | TypeScript |
|---|---|---|
| `useQuery` | `use_query`, `use_query_with`, `use_query_by`, `use_query_config` | `useQuery`, `createQuery` |
| `useMutation` | `use_mutation`, `use_mutation_with` | `useMutation`, `createMutation` |
| `useInfiniteQuery` | `use_infinite_query` | `useInfiniteQuery`, `createInfiniteQuery` |
| `useQueries` | `use_queries` | `useQueries`, `createQueries` |
| `useIsFetching`, `useIsMutating`, `useMutationState` | `use_is_fetching`, `use_is_mutating`, `use_mutation_state` | The same names |
| `status`, `fetchStatus` | `status_v5()`, `fetch_status()` | The same names |
| `staleTime`, `gcTime`, `retry`, `retryDelay` | `stale_time`, `gc_time`, `retry_if`, `retry_delay` | The same names |
| `QueryClient` | `query_client()`, then `invalidate_queries`, `set_query_data` and so on | `QueryClient`, `useQueryClient` |
| `focusManager`, `onlineManager` | `focus_manager()`, `online_manager()` | The same names |
| `dehydrate`, `hydrate`, persistence | `dehydrate`, `hydrate`, `persist_to_storage`, behind the `persist` feature | `dehydrate`, `hydrate`, `createStoragePersister`, `PersistQueryClientProvider` |
| `QueryCache`, `MutationCache` and their callbacks | `get_query_cache()`, `get_mutation_cache()`, `set_config` | The same as TanStack |
| Per-call callbacks | `mutate_with` | `mutate(variables, options)` |
| `throwOnError`, `QueryErrorResetBoundary` | `throw_on_error`, `use_query_error_reset_boundary`, with NSH's `catch_error` | The same names, with NSH's `ErrorBoundary` |
| `usePrefetchQuery`, `usePrefetchInfiniteQuery` | `use_prefetch_query`, `use_prefetch_infinite_query` | The same names |
| Data for a debug panel (from `gpui-query`) | `diagnostics()`, `use_diagnostics()` | `useQueryDiagnostics()` |

Focus means the app is visible, as in TanStack: `active` and `inactive` count as focused and `background` does not. Online is `true` until the app says otherwise, because NSH 0.4.0 has no network status API, so call `setOnline` or `set_online` yourself.

## State

Both versions follow NSH's rule that only the bindings that read a changed value run again. A binding on `data` does not rerun when a fetch starts, or when a different query changes. Tests with run counters prove this: `crates/nsh-query/tests/fine_grained.rs` for Rust, and `packages/nsh-query/tests/` for TypeScript.

## Where it differs from TanStack

**TypeScript:** an `nsh-query` query is TanStack's own code, so its behavior is TanStack's. Results read as properties, and a binding on `query.data` reruns only when `data` changes. The Suspense hooks throw `ERR_NSH_QUERY_NO_SUSPENSE`, because NSH 0.4.0 has no Suspense boundary. `packages/nsh-query/README.md` lists the rest.

**Rust:** these are deliberate:

- The default `stale_time` stays 60 seconds, as it was in this crate. TanStack's default is 0.
- `Query::status()` still returns `gpui-query`'s value. `status_v5()` and `fetch_status()` are TanStack's.
- `fetch_query` returns a `Result`. Its `Cancelled` error replaces TanStack's thrown one.
- `mutate_async` returns an `Option` that is always `Some`, kept for earlier callers.
- Options that carry a value type, such as `retry_if`, `initial_data` and `placeholder_data`, live on `QueryConfig<T, E>`, because `QueryOptions` has no type parameters.

Following TanStack changed these for code written against earlier versions of the crate:

- `InfiniteQuery::refetch()` reloads every loaded page and keeps them on screen. `reset()` is what `refetch()` used to do.
- Infinite lists live in the client cache, so reopening a screen within `gc_time` keeps its pages. Call `query_client().remove(key)` when a screen needs a first load.
- An infinite list has no page limit unless `max_pages` sets one. `gpui-query`'s core default of 50 no longer applies.
- `is_pending()`, `is_success()` and `is_failure()` follow TanStack's status, so `is_success()` stays true during a refetch.
- Entries made by `set_data`, `prefetch` or `fetch_query` leave the cache after `gc_time` if nothing observes them.

`docs/parity-typescript.md` and `docs/parity-rust.md` list every TanStack and `gpui-query` name, with its status and the test that covers it. Rows marked `not applicable` say why.

## Test

```sh
cargo test --all-features
cd packages/nsh-query && nsh check && nsh build && nsh test
```

The tests drive real UIs through the headless engine on a manual clock.
