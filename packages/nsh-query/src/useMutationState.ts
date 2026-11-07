import {
  type Mutation,
  type MutationFilters,
  type MutationState,
  type QueryClient,
  replaceEqualDeep,
} from "@tanstack/query-core";
import { type Accessor, cleanup, effect, signal, untracked } from "nsh";
import { resolveQueryClient } from "./QueryClientProvider";

/** What `useMutationState` reads: the filters, and an optional `select` of each mutation. */
export interface MutationStateOptions<TResult = MutationState> {
  filters?: MutationFilters;
  select?: (mutation: Mutation<any, any, any, any>) => TResult;
}

/**
 * The state of the mutations that match `options.filters`, as an accessor. The array is replaced
 * only when its contents change.
 */
export function useMutationState<TResult = MutationState>(
  options?: Accessor<MutationStateOptions<TResult>> | MutationStateOptions<TResult>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): Accessor<Array<TResult>> {
  const client = resolveQueryClient(queryClient);
  const cache = client.getMutationCache();
  const readOptions = typeof options === "function" ? options : () => options ?? {};

  const select = (current: MutationStateOptions<TResult>): TResult[] =>
    cache.findAll(current.filters).map((mutation) =>
      (current.select ? current.select(mutation) : mutation.state) as TResult,
    );

  const result = signal<TResult[]>(untracked(() => select(readOptions())));

  const unsubscribe = cache.subscribe(() => {
    untracked(() => {
      const next = replaceEqualDeep(result(), select(readOptions()));
      if (next !== result()) result.set(next);
    });
  });
  cleanup(unsubscribe);

  effect(() => {
    const next = readOptions();
    untracked(() => {
      const replaced = replaceEqualDeep(result(), select(next));
      if (replaced !== result()) result.set(replaced);
    });
  });

  return result;
}
