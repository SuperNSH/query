import {
  type DefaultError,
  type InfiniteData,
  InfiniteQueryObserver,
  type InfiniteQueryObserverResult,
  type QueryClient,
  type QueryKey,
} from "@tanstack/query-core";
import type { Accessor } from "nsh";
import { useBaseQuery } from "./useBaseQuery";
import type { UseInfiniteQueryOptions } from "./types";

/**
 * Subscribes to a paginated query: `fetchNextPage`, `fetchPreviousPage`, `hasNextPage` and the rest
 * of the infinite-query fields of the result.
 */
export function useInfiniteQuery<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = InfiniteData<TQueryFnData>,
  TQueryKey extends QueryKey = QueryKey,
  TPageParam = unknown,
>(
  options: Accessor<UseInfiniteQueryOptions<TQueryFnData, TError, TData, TQueryKey, TPageParam>> | UseInfiniteQueryOptions<TQueryFnData, TError, TData, TQueryKey, TPageParam>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): InfiniteQueryObserverResult<TData, TError> {
  return useBaseQuery<InfiniteQueryObserverResult<TData, TError>>(options as never, InfiniteQueryObserver as never, queryClient);
}

/** Alias of `useInfiniteQuery`. */
export const createInfiniteQuery = useInfiniteQuery;
