import {
  type DefaultError,
  type InfiniteQueryObserver,
  type QueryClient,
  type QueryObserver,
  type QueryObserverOptions,
  type QueryObserverResult,
  shouldThrowError,
} from "@tanstack/query-core";
import { type Accessor, cleanup, effect, signal, untracked } from "nsh";
import { NshQueryError, errorCodes } from "./errors";
import { useIsRestoring } from "./isRestoring";
import { useQueryErrorResetBoundary } from "./QueryErrorResetBoundary";
import { resolveQueryClient } from "./QueryClientProvider";
import { reactiveResult } from "./result";

/**
 * The shared body of `useQuery` and `useInfiniteQuery`: one core observer, subscribed while the
 * component is shown, with its options kept in step with the accessor.
 */
export function useBaseQuery<TResult>(
  options: Accessor<QueryObserverOptions<any, any, any, any, any, any>> | QueryObserverOptions<any, any, any, any, any, any>,
  Observer: typeof QueryObserver | typeof InfiniteQueryObserver,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): TResult {
  const client = resolveQueryClient(queryClient);
  const isRestoring = useIsRestoring();
  const errorReset = useQueryErrorResetBoundary();
  const readOptions = typeof options === "function" ? options : () => options;

  const defaulted = () => {
    const resolved = client.defaultQueryOptions(readOptions());
    resolved._optimisticResults = isRestoring() ? "isRestoring" : "optimistic";
    return resolved;
  };

  const initial = untracked(defaulted);
  if (initial.suspense) {
    throw new NshQueryError(
      errorCodes.noSuspense,
      "nsh-query: suspense is not supported, because NSH has no Suspense boundary. Read isPending instead.",
    );
  }

  const observer = new (Observer as typeof QueryObserver)(client, initial) as QueryObserver<any, any, any, any, any>;
  const current = signal<QueryObserverResult<any, any>>(observer.getOptimisticResult(initial));

  // Nothing subscribes while a persisted client restores: subscribing is what fetches on mount.
  let unsubscribe: (() => void) | undefined;
  const subscribeUnlessRestoring = () => {
    if (unsubscribe || untracked(isRestoring)) return;
    unsubscribe = observer.subscribe((next) => current.set(next));
  };
  subscribeUnlessRestoring();
  cleanup(() => {
    unsubscribe?.();
    unsubscribe = undefined;
  });

  effect(() => {
    const next = defaulted();
    untracked(() => {
      observer.setOptions(next);
      current.set(observer.getOptimisticResult(next));
      subscribeUnlessRestoring();
    });
  });

  // Thrown only to an error boundary, and not after a reset: the children then retry their queries.
  effect(() => {
    const result = current();
    if (errorReset.isReset()) return;
    if (result.isError && shouldThrowError(observer.options.throwOnError, [result.error as DefaultError, observer.getCurrentQuery()])) {
      throw result.error;
    }
  });

  return reactiveResult(current) as TResult;
}
