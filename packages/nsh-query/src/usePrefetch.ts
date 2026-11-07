import type { DefaultError, InfiniteData, QueryClient, QueryKey } from "@tanstack/query-core";
import { type Accessor, effect, untracked } from "nsh";
import { resolveQueryClient } from "./QueryClientProvider";
import type { UseInfiniteQueryOptions, UseQueryOptions } from "./types";

/**
 * Starts a `prefetchQuery` while the component is shown, and again when the options change. The fetch
 * is not awaited; `prefetchQuery` never rejects.
 */
export function usePrefetchQuery<TQueryFnData = unknown, TError = DefaultError, TData = TQueryFnData, TQueryKey extends QueryKey = QueryKey>(
  options: Accessor<UseQueryOptions<TQueryFnData, TError, TData, TQueryKey>> | UseQueryOptions<TQueryFnData, TError, TData, TQueryKey>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): void {
  const client = resolveQueryClient(queryClient);
  const readOptions = typeof options === "function" ? options : () => options;
  effect(() => {
    const next = readOptions();
    untracked(() => void client.prefetchQuery(next as never));
  });
}

/**
 * Starts a `prefetchInfiniteQuery` while the component is shown, and again when the options change.
 */
export function usePrefetchInfiniteQuery<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = InfiniteData<TQueryFnData>,
  TQueryKey extends QueryKey = QueryKey,
  TPageParam = unknown,
>(
  options: Accessor<UseInfiniteQueryOptions<TQueryFnData, TError, TData, TQueryKey, TPageParam>> | UseInfiniteQueryOptions<TQueryFnData, TError, TData, TQueryKey, TPageParam>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): void {
  const client = resolveQueryClient(queryClient);
  const readOptions = typeof options === "function" ? options : () => options;
  effect(() => {
    const next = readOptions();
    untracked(() => void client.prefetchInfiniteQuery(next as never));
  });
}
