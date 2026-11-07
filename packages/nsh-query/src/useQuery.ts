import { type DefaultError, type QueryClient, type QueryKey, QueryObserver, type QueryObserverResult, type DefinedQueryObserverResult } from "@tanstack/query-core";
import type { Accessor } from "nsh";
import { useBaseQuery } from "./useBaseQuery";
import type { DefinedInitialDataOptions, UndefinedInitialDataOptions, UseQueryOptions } from "./types";

/**
 * Subscribes to a query. With `initialData` always defined, `data` is never `undefined`.
 * @param options The TanStack `useQuery` options, as an accessor so a tracked key or `enabled` moves the query.
 * @param queryClient The client to use. Otherwise the nearest `QueryClientProvider`'s.
 */
export function useQuery<TQueryFnData = unknown, TError = DefaultError, TData = TQueryFnData, TQueryKey extends QueryKey = QueryKey>(
  options: Accessor<DefinedInitialDataOptions<TQueryFnData, TError, TData, TQueryKey>> | DefinedInitialDataOptions<TQueryFnData, TError, TData, TQueryKey>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): DefinedQueryObserverResult<TData, TError>;
export function useQuery<TQueryFnData = unknown, TError = DefaultError, TData = TQueryFnData, TQueryKey extends QueryKey = QueryKey>(
  options: Accessor<UndefinedInitialDataOptions<TQueryFnData, TError, TData, TQueryKey>> | UndefinedInitialDataOptions<TQueryFnData, TError, TData, TQueryKey>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): QueryObserverResult<TData, TError>;
export function useQuery<TQueryFnData = unknown, TError = DefaultError, TData = TQueryFnData, TQueryKey extends QueryKey = QueryKey>(
  options: Accessor<UseQueryOptions<TQueryFnData, TError, TData, TQueryKey>> | UseQueryOptions<TQueryFnData, TError, TData, TQueryKey>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): QueryObserverResult<TData, TError> {
  return useBaseQuery<QueryObserverResult<TData, TError>>(options as never, QueryObserver, queryClient);
}

/** Alias of `useQuery`, kept for the `create*` names of TanStack's Solid adapter. */
export const createQuery = useQuery;
