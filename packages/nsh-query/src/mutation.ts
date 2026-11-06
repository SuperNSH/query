import { type Accessor, signal } from "nsh";
import { type QueryClient, queryClient } from "./client";
import { type QueryKey, noRetries, type RetryPolicy, retryDelay } from "./policy";

export type MutationStatus = "idle" | "loading" | "success" | "failure";

export interface MutationOptions<V, T> {
  mutate: (variables: V, context: { signal: AbortSignal }) => Promise<T>;
  /** Default none, because a write that failed may still have reached the server. */
  retry?: RetryPolicy;
  /** Queries under these prefixes are invalidated after a success. */
  invalidates?: readonly QueryKey[];
  /** Runs before the write. The place for an optimistic `setData`. */
  onMutate?: (variables: V) => void;
  onSuccess?: (data: T, variables: V) => void;
  /** Runs after the last failed attempt. The place for `rollback`. */
  onError?: (error: unknown, variables: V) => void;
  /** Runs last, after a success or a failure. */
  onSettled?: (variables: V) => void;
  client?: QueryClient;
}

export interface Mutation<V, T> {
  /** Starts the write. A newer call supersedes this one: its result and callbacks are dropped. */
  mutate(variables: V): void;
  /** `mutate` that resolves with the result. Rejects with `MutationSuperseded` when superseded. */
  mutateAsync(variables: V): Promise<T>;
  status: Accessor<MutationStatus>;
  isLoading: Accessor<boolean>;
  isSuccess: Accessor<boolean>;
  isFailure: Accessor<boolean>;
  data: Accessor<T | undefined>;
  error: Accessor<unknown>;
  variables: Accessor<V | undefined>;
  /** Back to idle. A call in flight is dropped. */
  reset(): void;
}

/** What `mutateAsync` rejects with when a newer call replaced it. */
export class MutationSuperseded extends Error {
  constructor() {
    super("a newer mutation replaced this one");
    this.name = "MutationSuperseded";
  }
}

/** A write. A call in flight finishes even if the component goes, so saving and leaving works. */
export function createMutation<V, T>(options: MutationOptions<V, T>): Mutation<V, T> {
  const client = options.client ?? queryClient;
  const retry = options.retry ?? noRetries;
  const status = signal<MutationStatus>("idle");
  const data = signal<T | undefined>(undefined);
  const error = signal<unknown>(null);
  const variables = signal<V | undefined>(undefined);
  let generation = 0;
  let controller: AbortController | null = null;

  const run = async (input: V): Promise<T> => {
    const id = ++generation;
    controller?.abort();
    const own = new AbortController();
    controller = own;
    options.onMutate?.(input);
    variables.set(input);
    error.set(null);
    status.set("loading");

    let attempt = 0;
    for (;;) {
      try {
        const result = await options.mutate(input, { signal: own.signal });
        if (id !== generation) throw new MutationSuperseded();
        options.onSuccess?.(result, input);
        data.set(result);
        status.set("success");
        for (const prefix of options.invalidates ?? []) client.invalidate(prefix);
        options.onSettled?.(input);
        return result;
      } catch (failure) {
        if (failure instanceof MutationSuperseded || id !== generation) throw new MutationSuperseded();
        if (attempt < retry.maxRetries) {
          const delay = retryDelay(retry, attempt);
          attempt += 1;
          await new Promise((resolve) => setTimeout(resolve, delay));
          if (id !== generation) throw new MutationSuperseded();
          continue;
        }
        options.onError?.(failure, input);
        data.set(undefined);
        error.set(failure);
        status.set("failure");
        options.onSettled?.(input);
        throw failure;
      }
    }
  };

  return {
    mutate(input) {
      run(input).catch(() => {});
    },
    mutateAsync: run,
    status,
    isLoading: () => status() === "loading",
    isSuccess: () => status() === "success",
    isFailure: () => status() === "failure",
    data,
    error,
    variables,
    reset() {
      generation += 1;
      controller?.abort();
      controller = null;
      status.set("idle");
      data.set(undefined);
      error.set(null);
      variables.set(undefined);
    },
  };
}
