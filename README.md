# nsh-query

Async state for [Native Shell](https://nsh.dev) apps, in the style of TanStack Query: caching, deduplication,
retries, invalidation, mutations and infinite lists, exposed as signals so a binding updates by itself.

There are two versions with the same behavior:

| Version | Path | For |
|---|---|---|
| Rust crate | `crates/nsh-query` | Rust and Rust-with-TSX apps |
| TypeScript package | `packages/nsh-query` | TSX apps, signed packages, anything with no Rust crate |

## Why two versions

The Rust crate reuses [`gpui-query`](https://github.com/freeoxide/gpui-query): its `core` feature is a state machine
with no GPUI dependency (`QueryResource`, `MutationResource`, `InfiniteQueryResource`, `CachePolicy`, `RetryPolicy`,
`QueryKey`). This repo replaces its GPUI `client` and `hook` layers with NSH signals, tasks and timers.

That cannot serve TSX. A TSX app runs on NSH's prebuilt runtime in QuickJS with no crate of its own, and generic
Rust values and JS closures do not cross the module boundary. So TSX gets a TypeScript port of the same state
machine, with an `AbortSignal` for cancellation.

Requires NSH 0.4.0.

## Rust

```toml
[dependencies]
nsh-query = { path = "../nsh-query/crates/nsh-query" }
```

```rust
use nsh_core::prelude::*;
use nsh_query::{MutationOptions, QueryOptions, use_mutation_with, use_query_with};

#[component]
pub fn Todos() -> impl IntoElement {
    let todos = use_query_with::<Vec<String>, String, _, _>(
        "todos",
        QueryOptions::new().ttl(std::time::Duration::from_secs(30)),
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

## TypeScript

Install it into `node_modules`, which the NSH bundler and `nsh check` both read:

```sh
mkdir -p node_modules && ln -s ../../nsh-query/packages/nsh-query node_modules/nsh-query
```

```tsx
import { Text, View } from "nsh";
import { createMutation, createQuery } from "nsh-query";

export function Todos() {
  const todos = createQuery<string[]>({
    key: "todos",
    cache: { ttlMs: 30_000 },
    fetch: async ({ signal }) => (await fetch("https://api.example.com/todos", { signal })).json(),
  });
  const add = createMutation<string, void>({
    invalidates: ["todos"],
    mutate: async (title, { signal }) => {
      await fetch("https://api.example.com/todos", { method: "POST", body: JSON.stringify({ title }), signal });
    },
  });
  return (
    <View>
      <Text>{`${todos.data()?.length ?? 0} todos`}</Text>
      <Text role="button" onPress={() => add.mutate("Walk")}>Add</Text>
    </View>
  );
}
```

## Behavior

| Feature | Rust | TypeScript |
|---|---|---|
| Observe a key | `use_query`, `use_query_with` | `createQuery` |
| A key that changes | `use_query_by(move \|\| key(id.get()), ..)` | `key: () => ["users", id()]` |
| Freshness | `CachePolicy::Ttl`, `StaleWhileRevalidate`, `NoCache` | `cache: { ttlMs, staleMs }`, `"none"` |
| Retries | `RetryPolicy::new(3).with_delay(1000)` | `retry: { maxRetries: 3, delayMs: 1000 }` |
| Writes | `use_mutation_with` | `createMutation` |
| Optimistic update | `query.set_data`, `query.rollback` | `query.setData`, `query.rollback` |
| Paging | `use_infinite_query` | `createInfiniteQuery` |
| Cache access | `query_client()` | `queryClient`, `setQueryData`, `getQueryData`, `prefetchQuery` |
| Conditional | `QueryOptions::enabled` | `enabled` |
| Polling | `refetch_interval` | `refetchIntervalMs` |
| Foreground refetch | `refetch_on_foreground` | `refetchOnForeground` |

Two observers of one key share one load. A load that a newer one replaced has its answer dropped. A retry stays
loading, so a screen never flashes an error between attempts. An entry nothing observes is dropped after its GC
time, five minutes by default. A mutation in flight finishes after its component goes, so saving and leaving works.

## Test

```sh
cargo test
cd packages/nsh-query && nsh check && nsh build && nsh test
```

The tests drive real UIs through the headless engine on a manual clock.
