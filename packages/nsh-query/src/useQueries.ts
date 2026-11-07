import {
  type DefaultError,
  type QueryClient,
  type QueryObserverOptions,
  type QueryObserverResult,
  QueriesObserver,
} from "@tanstack/query-core";
import { type Accessor, batch, cleanup, computed, effect, signal, untracked } from "nsh";
import { useIsRestoring } from "./isRestoring";
import { resolveQueryClient } from "./QueryClientProvider";

type AnyOptions = QueryObserverOptions<any, any, any, any, any, any>;

/** The result of one query in `useQueries`, typed from its options. */
export type QueryResultFor<O> = O extends QueryObserverOptions<infer TFnData, infer TError, infer TData, any, any, any>
  ? QueryObserverResult<TData, TError>
  : QueryObserverResult<unknown, DefaultError>;

/** The results of `useQueries`, one per query, in order. */
export type QueriesResults<T extends readonly unknown[]> = { [K in keyof T]: QueryResultFor<T[K]> };

/** Options of `useQueries`: the queries, and an optional `combine` of their results. */
export interface UseQueriesOptions<T extends readonly unknown[], TCombinedResult = QueriesResults<T>> {
  queries: readonly [...T];
  combine?: (results: QueriesResults<T>) => TCombinedResult;
}

/**
 * Runs several queries together. Without `combine` the result is a list: `queries[i]` is query `i`, and
 * `queries[i].data` wakes only when that query's data changes. With `combine` the result is the combined
 * value, and a read of it wakes when any part of it changes. Whether `combine` is given is read once.
 */
export function useQueries<T extends AnyOptions[], TCombinedResult = QueriesResults<T>>(
  options: Accessor<UseQueriesOptions<T, TCombinedResult>> | UseQueriesOptions<T, TCombinedResult>,
  queryClient?: QueryClient | Accessor<QueryClient | undefined>,
): TCombinedResult {
  const client = resolveQueryClient(queryClient);
  const isRestoring = useIsRestoring();
  const readOptions = typeof options === "function" ? options : () => options;
  const readCombine = () => readOptions().combine as ((results: unknown) => unknown) | undefined;

  const defaulted = () =>
    ([...readOptions().queries] as AnyOptions[]).map((query) => {
      const resolved = client.defaultQueryOptions(query);
      resolved._optimisticResults = isRestoring() ? "isRestoring" : "optimistic";
      return resolved;
    });

  const observer = new QueriesObserver<unknown>(client, untracked(defaulted), untracked(() => ({ combine: readCombine() })));
  const combinedNow = (raw?: QueryObserverResult[]) =>
    observer.getOptimisticResult(untracked(defaulted), untracked(readCombine) as never)[1](raw) as unknown;
  const current = signal<unknown>(untracked(() => combinedNow()));

  let unsubscribe: (() => void) | undefined;
  const subscribeUnlessRestoring = () => {
    if (unsubscribe || untracked(isRestoring)) return;
    unsubscribe = observer.subscribe((raw) => current.set(combinedNow(raw as QueryObserverResult[])));
  };
  subscribeUnlessRestoring();
  cleanup(() => {
    unsubscribe?.();
    unsubscribe = undefined;
  });

  effect(() => {
    const queries = defaulted();
    const combine = readCombine();
    untracked(() => {
      observer.setQueries(queries, { combine: combine as never });
      current.set(combinedNow());
      subscribeUnlessRestoring();
    });
  });

  if (untracked(readCombine)) return valueView(current) as TCombinedResult;

  // One memo per index and field, made when the list length changes. Made here rather than in a
  // binding, because a memo made inside a binding stops updating after the binding's first run.
  const results = (): QueryObserverResult[] => current() as QueryObserverResult[];
  const count = computed(() => results().length);
  let built = -1;
  const views = signal(untracked(() => buildViews(results, count())));
  built = untracked(count);
  effect(() => {
    const length = count();
    untracked(() => {
      if (length === built) return;
      built = length;
      batch(() => views.set(buildViews(results, length)));
    });
  });
  return listView(views) as TCombinedResult;
}

/** One view per query: each field is a memo over that query's result at that index. */
function buildViews(results: () => QueryObserverResult[], length: number): Array<Record<string, unknown>> {
  const views: Array<Record<string, unknown>> = [];
  for (let index = 0; index < length; index++) {
    const view: Record<string, unknown> = {};
    for (const key of Object.keys(untracked(() => results()[index]!))) {
      const field = computed(() => results()[index]?.[key as keyof QueryObserverResult]);
      Object.defineProperty(view, key, { enumerable: true, get: () => field() });
    }
    views.push(view);
  }
  return views;
}

/** An array of the views. Reading an index or the length reads the views signal, which changes with the list. */
function listView(views: Accessor<Array<Record<string, unknown>>>): unknown[] {
  const isIndex = (key: PropertyKey): key is string => typeof key === "string" && /^\d+$/.test(key);
  return new Proxy([] as unknown[], {
    get(target, key) {
      if (key === "length") return views().length;
      if (isIndex(key)) return views()[Number(key)];
      return Reflect.get(target, key);
    },
    has(target, key) {
      if (isIndex(key)) return Number(key) < views().length;
      return Reflect.has(target, key);
    },
  });
}

/** A combined value read through `source`: reads follow the whole value. */
function valueView(source: Accessor<unknown>): object {
  return new Proxy({} as object, {
    get: (_target, key) => Reflect.get(source() as object, key),
    has: (_target, key) => Reflect.has(source() as object, key),
    ownKeys: () => Reflect.ownKeys(source() as object),
    getOwnPropertyDescriptor: (_target, key) => {
      const descriptor = Reflect.getOwnPropertyDescriptor(source() as object, key);
      return descriptor ? { ...descriptor, configurable: true } : undefined;
    },
  });
}

/** Alias of `useQueries`. */
export const createQueries = useQueries;
