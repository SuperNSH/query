import type { QueryClient, QueryFilters } from "@tanstack/query-core";
import { type Accessor, cleanup, effect, signal, untracked } from "nsh";
import { resolveQueryClient } from "./QueryClientProvider";

/**
 * The number of queries that match `filters` and are fetching, as an accessor. `filters` may be an
 * accessor, so the count follows the filters it reads.
 */
export function useIsFetching(
  filters?: QueryFilters | Accessor<QueryFilters>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): Accessor<number> {
  const client = resolveQueryClient(queryClient);
  const readFilters = typeof filters === "function" ? filters : () => filters;
  const count = signal(untracked(() => client.isFetching(readFilters())));

  const unsubscribe = client.getQueryCache().subscribe(() => {
    untracked(() => count.set(client.isFetching(readFilters())));
  });
  cleanup(unsubscribe);

  effect(() => {
    const next = readFilters();
    untracked(() => count.set(client.isFetching(next)));
  });

  return count;
}
