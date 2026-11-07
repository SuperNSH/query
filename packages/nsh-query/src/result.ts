import { type Accessor, computed, untracked } from "nsh";

/**
 * A result object whose every field is a getter over a memo of that one field. A binding that reads
 * `result.data` runs again only when `data` changes, not when `fetchStatus` does. `fixed` values are
 * plain properties that are not read from `source`.
 */
export function reactiveResult<T extends object>(source: Accessor<T>, fixed: Partial<T> = {}): T {
  const result: Record<string, unknown> = {};
  for (const key of Object.keys(untracked(source))) {
    if (key in fixed) continue;
    const field = computed(() => (source() as Record<string, unknown>)[key]);
    Object.defineProperty(result, key, { enumerable: true, get: () => field() });
  }
  for (const [key, value] of Object.entries(fixed)) {
    Object.defineProperty(result, key, { enumerable: true, value });
  }
  return result as T;
}
