import { type Accessor, context } from "nsh";

const IsRestoringContext = context<Accessor<boolean>>(() => false);

/** Provides whether a persisted client is being restored. `PersistQueryClientProvider` uses it. */
export const IsRestoringProvider = IsRestoringContext.Provider;

/** Reads, as an accessor, whether a persisted client is being restored. Queries wait for it. */
export function useIsRestoring(): Accessor<boolean> {
  return IsRestoringContext();
}
