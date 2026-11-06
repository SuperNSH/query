import { keyId, type QueryKey, keyStartsWith } from "./policy";

export interface AnyEntry {
  readonly key: QueryKey;
  invalidate(): void;
  cancel(): void;
  dispose(): void;
}

/** A query cache. Apps usually share the default `queryClient`. */
export class QueryClient {
  /** @internal */
  entries = new Map<string, AnyEntry>();

  private matching(prefix: QueryKey): AnyEntry[] {
    return [...this.entries.values()].filter((entry) => keyStartsWith(entry.key, prefix));
  }

  /** Marks entries under `prefix` stale and reloads the observed ones, replacing a load in flight. */
  invalidate(prefix: QueryKey): void {
    for (const entry of this.matching(prefix)) entry.invalidate();
  }

  /** Stops loads under `prefix`. Their fetchers' `AbortSignal`s fire, and loaded data stays. */
  cancel(prefix: QueryKey): void {
    for (const entry of this.matching(prefix)) entry.cancel();
  }

  /** Drops entries under `prefix`. A new observer starts from nothing. */
  remove(prefix: QueryKey): void {
    for (const entry of this.matching(prefix)) {
      this.entries.delete(keyId(entry.key));
      entry.dispose();
    }
  }

  /** Drops every entry. */
  clear(): void {
    for (const entry of this.entries.values()) entry.dispose();
    this.entries.clear();
  }
}

/** The client `createQuery` and `createMutation` use unless given another. */
export const queryClient = new QueryClient();
