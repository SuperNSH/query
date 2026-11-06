/** A key: a string, or segments such as `["todos", id]`. Segments compare as strings. */
export type QueryKey = string | readonly (string | number)[];

/**
 * `"none"` reloads on every mount. `{ ttlMs }` is fresh for that long.
 * `{ ttlMs, staleMs }` then serves the old data while it reloads, for another `staleMs`.
 */
export type CachePolicy = "none" | { ttlMs: number; staleMs?: number };

export interface RetryPolicy {
  /** 0 means no retries. */
  maxRetries: number;
  /** Delay before a retry. Default 1000. */
  delayMs?: number;
  /** Doubles the delay per attempt, up to `maxDelayMs`. */
  exponentialBackoff?: boolean;
  /** Default 30000. */
  maxDelayMs?: number;
}

export const noRetries: RetryPolicy = { maxRetries: 0 };

/** Three retries with backoff from one second. */
export const defaultRetry: RetryPolicy = { maxRetries: 3, delayMs: 1000, exponentialBackoff: true, maxDelayMs: 30_000 };

export const defaultCache: CachePolicy = { ttlMs: 60_000 };

export type QueryStatus = "idle" | "loading-empty" | "loading-with-data" | "success" | "failure";

export type RequestPolicy = "ignore-while-loading" | "latest-wins";

export function retryDelay(policy: RetryPolicy, attempt: number): number {
  const base = policy.delayMs ?? 1000;
  if (!policy.exponentialBackoff) return base;
  return Math.min(base * 2 ** Math.min(attempt, 30), policy.maxDelayMs ?? 30_000, 3_600_000);
}

export function keySegments(key: QueryKey): string[] {
  return typeof key === "string" ? [key] : key.map(String);
}

export function keyId(key: QueryKey): string {
  return JSON.stringify(keySegments(key));
}

export function keyStartsWith(key: QueryKey, prefix: QueryKey): boolean {
  const segments = keySegments(key);
  return keySegments(prefix).every((segment, index) => segments[index] === segment);
}

export function isFresh(policy: CachePolicy, ageMs: number): boolean {
  return policy !== "none" && ageMs <= policy.ttlMs;
}

export function isStaleButServeable(policy: CachePolicy, ageMs: number): boolean {
  return policy !== "none" && policy.staleMs !== undefined && ageMs > policy.ttlMs && ageMs <= policy.ttlMs + policy.staleMs;
}
