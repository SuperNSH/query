import type {
  DefaultError,
  InfiniteData,
  InitialDataFunction,
  NonUndefinedGuard,
  QueryKey,
  QueryKeyWithDataTag,
} from "@tanstack/query-core";
import type { DefinedInitialDataOptions, UndefinedInitialDataOptions, UseInfiniteQueryOptions, UseMutationOptions, UseQueryOptions } from "./types";

/** Returns `options` typed for `useQuery`, with `initialData` known to be defined. */
export function queryOptions<TQueryFnData = unknown, TError = DefaultError, TData = TQueryFnData, TQueryKey extends QueryKey = QueryKey>(
  options: DefinedInitialDataOptions<TQueryFnData, TError, TData, TQueryKey>,
): DefinedInitialDataOptions<TQueryFnData, TError, TData, TQueryKey> & QueryKeyWithDataTag<TQueryKey, TQueryFnData, TError>;
export function queryOptions<TQueryFnData = unknown, TError = DefaultError, TData = TQueryFnData, TQueryKey extends QueryKey = QueryKey>(
  options: UndefinedInitialDataOptions<TQueryFnData, TError, TData, TQueryKey>,
): UndefinedInitialDataOptions<TQueryFnData, TError, TData, TQueryKey> & QueryKeyWithDataTag<TQueryKey, TQueryFnData, TError>;
export function queryOptions(options: UseQueryOptions<any, any, any, any>): unknown {
  return options;
}

/** Returns `options` typed for `useInfiniteQuery`. */
export function infiniteQueryOptions<TQueryFnData, TError = DefaultError, TData = InfiniteData<TQueryFnData>, TQueryKey extends QueryKey = QueryKey, TPageParam = unknown>(
  options: UseInfiniteQueryOptions<TQueryFnData, TError, TData, TQueryKey, TPageParam> & {
    initialData?: undefined | NonUndefinedGuard<InfiniteData<TQueryFnData, TPageParam>> | InitialDataFunction<NonUndefinedGuard<InfiniteData<TQueryFnData, TPageParam>>>;
  },
): UseInfiniteQueryOptions<TQueryFnData, TError, TData, TQueryKey, TPageParam> & QueryKeyWithDataTag<TQueryKey, InfiniteData<TQueryFnData>, TError>;
export function infiniteQueryOptions(options: UseInfiniteQueryOptions<any, any, any, any, any>): unknown {
  return options;
}

/** Returns `options` typed for `useMutation`. */
export function mutationOptions<TData = unknown, TError = DefaultError, TVariables = void, TOnMutateResult = unknown>(
  options: UseMutationOptions<TData, TError, TVariables, TOnMutateResult>,
): UseMutationOptions<TData, TError, TVariables, TOnMutateResult> {
  return options;
}
