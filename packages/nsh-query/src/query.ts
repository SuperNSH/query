import { type Accessor, cleanup, computed, effect, lifecycle, signal, untracked } from "nsh";
import { type AnyEntry, type QueryClient, queryClient } from "./client";
import {
  type CachePolicy,
  defaultCache,
  defaultRetry,
  isFresh,
  keyId,
  keySegments,
  type QueryKey,
  type QueryStatus,
  type RequestPolicy,
  type RetryPolicy,
  retryDelay,
} from "./policy";

/** What a fetcher gets. Pass `signal` to `fetch` so a replaced load stops. */
export interface QueryContext {
  key: string[];
  signal: AbortSignal;
}

interface State<T> {
  status: QueryStatus;
  data: T | undefined;
  previous: T | undefined;
  error: unknown;
  updatedAt: number | null;
  retryCount: number;
}

type Fetcher<T> = (context: QueryContext) => Promise<T>;

class Entry<T> implements AnyEntry {
  state = signal<State<T>>({
    status: "idle",
    data: undefined,
    previous: undefined,
    error: null,
    updatedAt: null,
    retryCount: 0,
  });
  fetcher: Fetcher<T> | undefined;
  observers = 0;
  private request = 0;
  private active: number | null = null;
  private controller: AbortController | null = null;
  private gcTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(
    readonly key: QueryKey,
    private readonly client: QueryClient,
    public cache: CachePolicy,
    public retry: RetryPolicy,
    public requestPolicy: RequestPolicy,
    public gcMs: number,
  ) {}

  private patch(change: Partial<State<T>>): void {
    this.state.set({ ...this.state(), ...change });
  }

  private ageMs(now: number): number | null {
    const { updatedAt } = this.state();
    return updatedAt === null ? null : now - updatedAt;
  }

  /** Loads unless fresh. `force` skips the freshness check. */
  fetch(force = false): void {
    const now = Date.now();
    const age = this.ageMs(now);
    const hasData = this.state().data !== undefined;
    if (!force && hasData && age !== null && isFresh(this.cache, age)) return;
    if (this.active !== null && this.requestPolicy === "ignore-while-loading") return;
    this.start();
  }

  /** Starts a load now, replacing one in flight. */
  private start(): void {
    this.controller?.abort();
    const controller = new AbortController();
    this.controller = controller;
    const id = ++this.request;
    this.active = id;
    this.patch({ status: this.state().data !== undefined ? "loading-with-data" : "loading-empty", error: null });
    void this.attempts(id, controller);
  }

  private async attempts(id: number, controller: AbortController): Promise<void> {
    const fetcher = this.fetcher;
    if (!fetcher) return;
    let attempt = 0;
    for (;;) {
      try {
        const data = await fetcher({ key: keySegments(this.key), signal: controller.signal });
        if (this.active !== id) return;
        this.active = null;
        this.patch({
          status: "success",
          previous: this.state().data,
          data,
          error: null,
          updatedAt: Date.now(),
          retryCount: 0,
        });
        return;
      } catch (error) {
        if (this.active !== id) return;
        if (attempt < this.retry.maxRetries) {
          const delay = retryDelay(this.retry, attempt);
          attempt += 1;
          this.patch({ retryCount: this.state().retryCount + 1 });
          await new Promise((resolve) => setTimeout(resolve, delay));
          if (this.active !== id) return;
          continue;
        }
        this.active = null;
        this.patch({ status: "failure", error, updatedAt: Date.now() });
        return;
      }
    }
  }

  invalidate(): void {
    this.patch({ updatedAt: null });
    if (this.observers > 0) this.start();
  }

  cancel(): void {
    this.controller?.abort();
    this.controller = null;
    if (this.active === null) return;
    this.active = null;
    const { data } = this.state();
    this.patch({ status: data !== undefined ? "success" : "idle" });
  }

  dispose(): void {
    this.cancel();
    clearTimeout(this.gcTimer);
  }

  observe(): void {
    this.observers += 1;
    clearTimeout(this.gcTimer);
  }

  release(): void {
    this.observers = Math.max(0, this.observers - 1);
    if (this.observers > 0) return;
    this.gcTimer = setTimeout(() => this.client.remove(this.key), this.gcMs);
  }

  /** A fresh result, as if a load just finished. Replaces a load in flight. */
  commit(data: T): void {
    this.controller?.abort();
    this.active = null;
    this.patch({ status: "success", previous: this.state().data, data, error: null, updatedAt: Date.now() });
  }

  /** Keeps the timestamp, and the old value for `rollback`. */
  write(data: T): void {
    this.patch({ previous: this.state().data, data });
  }

  rollback(): boolean {
    const { previous } = this.state();
    if (previous === undefined) return false;
    this.patch({ data: previous, previous: undefined, status: "success", error: null });
    return true;
  }
}

function entryFor<T>(client: QueryClient, key: QueryKey, options: EntryOptions): Entry<T> {
  const id = keyId(key);
  const found = client.entries.get(id);
  if (found) return found as Entry<T>;
  const entry = new Entry<T>(
    key,
    client,
    options.cache ?? defaultCache,
    options.retry ?? defaultRetry,
    options.request ?? "ignore-while-loading",
    options.gcMs ?? 300_000,
  );
  client.entries.set(id, entry);
  return entry;
}

interface EntryOptions {
  cache?: CachePolicy;
  retry?: RetryPolicy;
  request?: RequestPolicy;
  /** How long an entry with no observers stays cached. Default five minutes. */
  gcMs?: number;
}

export interface QueryOptions<T> extends EntryOptions {
  /** The key, or a function of signals when it can change. */
  key: QueryKey | (() => QueryKey);
  fetch: Fetcher<T>;
  /** Nothing loads while this returns false. Tracked. */
  enabled?: () => boolean;
  /** Reloads this often while observed and not in the background. */
  refetchIntervalMs?: number;
  /** What to do on returning to the foreground. Default `"if-stale"`. */
  refetchOnForeground?: "always" | "if-stale" | "never";
  client?: QueryClient;
}

export interface Query<T> {
  data: Accessor<T | undefined>;
  error: Accessor<unknown>;
  status: Accessor<QueryStatus>;
  /** True until the first load ends: there is nothing to show yet. */
  isPending: Accessor<boolean>;
  /** True during any load, including a background refetch of existing data. */
  isFetching: Accessor<boolean>;
  isSuccess: Accessor<boolean>;
  isFailure: Accessor<boolean>;
  retryCount: Accessor<number>;
  updatedAt: Accessor<number | null>;
  /** Loads again now, ignoring freshness. */
  refetch(): void;
  /** Replaces the data, keeping the old value for `rollback`. For optimistic updates. */
  setData(data: T): void;
  /** Puts back the value `setData` replaced. False when there is none. */
  rollback(): boolean;
  /** A memo of part of the data. Bindings on it run only when that part changes. */
  select<U>(pick: (data: T) => U): Accessor<U | undefined>;
}

/** Observes a key. Call in a component, whose cleanup ends the observation. */
export function createQuery<T>(options: QueryOptions<T>): Query<T> {
  const client = options.client ?? queryClient;
  const readKey = (): QueryKey => (typeof options.key === "function" ? options.key() : options.key);

  const attach = (key: QueryKey): Entry<T> => {
    const entry = entryFor<T>(client, key, options);
    entry.fetcher = options.fetch;
    entry.observe();
    return entry;
  };

  const first = attach(untracked(readKey));
  const current = signal<Entry<T>>(first);
  let observed = first;

  effect(() => {
    const key = readKey();
    const on = options.enabled ? options.enabled() : true;
    untracked(() => {
      if (keyId(key) !== keyId(observed.key)) {
        const previous = observed;
        observed = attach(key);
        current.set(observed);
        previous.release();
      }
      if (on) observed.fetch();
    });
  });

  cleanup(() => observed.release());

  if (options.refetchIntervalMs !== undefined) {
    const timer = setInterval(() => {
      if (untracked(lifecycle) !== "background") observed.fetch(true);
    }, options.refetchIntervalMs);
    cleanup(() => clearInterval(timer));
  }

  const onForeground = options.refetchOnForeground ?? "if-stale";
  if (onForeground !== "never") {
    let was = untracked(lifecycle);
    effect(() => {
      const now = lifecycle();
      const before = was;
      was = now;
      if (now === "active" && before === "background") untracked(() => observed.fetch(onForeground === "always"));
    });
  }

  const state = () => current().state();
  const status = () => state().status;
  const data = () => state().data;

  return {
    data,
    error: () => state().error,
    status,
    isPending: () => status() === "idle" || status() === "loading-empty",
    isFetching: () => status() === "loading-empty" || status() === "loading-with-data",
    isSuccess: () => status() === "success",
    isFailure: () => status() === "failure",
    retryCount: () => state().retryCount,
    updatedAt: () => state().updatedAt,
    refetch: () => observed.fetch(true),
    setData: (value) => observed.write(value),
    rollback: () => observed.rollback(),
    select<U>(pick: (value: T) => U) {
      return computed(() => {
        const value = data();
        return value === undefined ? undefined : pick(value);
      });
    },
  };
}

/** The cached value of `key`, untracked. `undefined` when nothing is cached. */
export function getQueryData<T>(key: QueryKey, client: QueryClient = queryClient): T | undefined {
  const found = client.entries.get(keyId(key)) as Entry<T> | undefined;
  return found ? untracked(() => found.state().data) : undefined;
}

/** Writes `data` into the cache as a fresh result, creating the entry if needed. */
export function setQueryData<T>(key: QueryKey, data: T, client: QueryClient = queryClient): void {
  entryFor<T>(client, key, {}).commit(data);
}

/** Loads `key` into the cache with no observer, unless it is still fresh. */
export function prefetchQuery<T>(options: EntryOptions & { key: QueryKey; fetch: Fetcher<T>; client?: QueryClient }): void {
  const entry = entryFor<T>(options.client ?? queryClient, options.key, options);
  entry.fetcher = options.fetch;
  entry.fetch();
}
