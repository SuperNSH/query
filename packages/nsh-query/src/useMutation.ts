import {
  type DefaultError,
  MutationObserver,
  type QueryClient,
  noop,
  shouldThrowError,
  type MutateFunctionRest,
  type MutationObserverResult,
} from "@tanstack/query-core";
import { type Accessor, cleanup, effect, signal, untracked } from "nsh";
import { reactiveResult } from "./result";
import { resolveQueryClient } from "./QueryClientProvider";
import type { UseMutateFunction, UseMutationOptions, UseMutationResult } from "./types";

/**
 * A mutation: `mutate` starts it, `mutateAsync` returns its promise, and the result fields follow it.
 * The mutation keeps running when the component goes; only the subscription ends.
 */
export function useMutation<TData = unknown, TError = DefaultError, TVariables = void, TOnMutateResult = unknown>(
  options: Accessor<UseMutationOptions<TData, TError, TVariables, TOnMutateResult>> | UseMutationOptions<TData, TError, TVariables, TOnMutateResult>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): UseMutationResult<TData, TError, TVariables, TOnMutateResult> {
  const client = resolveQueryClient(queryClient);
  const readOptions = typeof options === "function" ? options : () => options;

  const observer = new MutationObserver<TData, TError, TVariables, TOnMutateResult>(client, untracked(readOptions));
  const current = signal<MutationObserverResult<TData, TError, TVariables, TOnMutateResult>>(observer.getCurrentResult());

  const unsubscribe = observer.subscribe((next) => current.set(next));
  cleanup(unsubscribe);

  effect(() => {
    const next = readOptions();
    untracked(() => observer.setOptions(next));
  });

  effect(() => {
    const result = current();
    if (result.isError && shouldThrowError(observer.options.throwOnError, [result.error as TError])) {
      throw result.error;
    }
  });

  const mutate: UseMutateFunction<TData, TError, TVariables, TOnMutateResult> = (...args) => {
    observer.mutate(args[0] as TVariables, args[1]).catch(noop);
  };
  const mutateAsync = (...args: MutateFunctionRest<TData, TError, TVariables, TOnMutateResult>) =>
    observer.mutate(args[0] as TVariables, args[1]);

  return reactiveResult(current as Accessor<object>, { mutate, mutateAsync }) as UseMutationResult<TData, TError, TVariables, TOnMutateResult>;
}

/** Alias of `useMutation`. */
export const createMutation = useMutation;
