import type {
  DefaultError,
  InfiniteData,
  InfiniteQueryObserverOptions,
  InitialDataFunction,
  MutationObserverOptions,
  MutateFunction,
  MutateFunctionRest,
  MutationObserverResult,
  NonUndefinedGuard,
  QueryKey,
  QueryObserverOptions,
} from "@tanstack/query-core";

/** Options accepted by `useQuery`: the TanStack Query options, unchanged. */
export type UseQueryOptions<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
> = QueryObserverOptions<TQueryFnData, TError, TData, TQueryFnData, TQueryKey>;

/** Options with an `initialData` that is always defined, so `data` is never `undefined`. */
export type DefinedInitialDataOptions<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
> = UseQueryOptions<TQueryFnData, TError, TData, TQueryKey> & {
  initialData: NonUndefinedGuard<TQueryFnData> | (() => NonUndefinedGuard<TQueryFnData>);
};

/** Options whose `initialData` may be absent. */
export type UndefinedInitialDataOptions<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
> = UseQueryOptions<TQueryFnData, TError, TData, TQueryKey> & {
  initialData?: undefined | InitialDataFunction<NonUndefinedGuard<TQueryFnData>> | NonUndefinedGuard<TQueryFnData>;
};

/** Options accepted by `useInfiniteQuery`. */
export type UseInfiniteQueryOptions<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = InfiniteData<TQueryFnData>,
  TQueryKey extends QueryKey = QueryKey,
  TPageParam = unknown,
> = InfiniteQueryObserverOptions<TQueryFnData, TError, TData, TQueryKey, TPageParam>;

/** Options accepted by `useMutation`. */
export type UseMutationOptions<
  TData = unknown,
  TError = DefaultError,
  TVariables = void,
  TOnMutateResult = unknown,
> = MutationObserverOptions<TData, TError, TVariables, TOnMutateResult>;

/** `mutate` of a mutation: starts it and never rejects; use `mutateAsync` to get the error. */
export type UseMutateFunction<
  TData = unknown,
  TError = DefaultError,
  TVariables = void,
  TOnMutateResult = unknown,
> = (...args: MutateFunctionRest<TData, TError, TVariables, TOnMutateResult>) => void;

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

/** A mutation result: the TanStack fields, with `mutate` that never rejects and `mutateAsync` that does. */
export type UseMutationResult<
  TData = unknown,
  TError = DefaultError,
  TVariables = void,
  TOnMutateResult = unknown,
> = DistributiveOmit<MutationObserverResult<TData, TError, TVariables, TOnMutateResult>, "mutate"> & {
  mutate: UseMutateFunction<TData, TError, TVariables, TOnMutateResult>;
  mutateAsync: MutateFunction<TData, TError, TVariables, TOnMutateResult>;
};

