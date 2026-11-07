import type { MutationFilters, QueryClient } from "@tanstack/query-core";
import { type Accessor, cleanup, effect, signal, untracked } from "nsh";
import { resolveQueryClient } from "./QueryClientProvider";

/**
 * The number of mutations that match `filters` and are pending, as an accessor. `filters` may be an
 * accessor, so the count follows the filters it reads.
 */
export function useIsMutating(
  filters?: MutationFilters | Accessor<MutationFilters>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): Accessor<number> {
  const client = resolveQueryClient(queryClient);
  const readFilters = typeof filters === "function" ? filters : () => filters;
  const count = signal(untracked(() => client.isMutating(readFilters())));

  const unsubscribe = client.getMutationCache().subscribe(() => {
    untracked(() => count.set(client.isMutating(readFilters())));
  });
  cleanup(unsubscribe);

  effect(() => {
    const next = readFilters();
    untracked(() => count.set(client.isMutating(next)));
  });

  return count;
}
