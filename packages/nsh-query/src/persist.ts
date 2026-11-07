import { createAsyncStoragePersister } from "@tanstack/query-async-storage-persister";
import {
  type PersistQueryClientOptions,
  persistQueryClientRestore,
  persistQueryClientSubscribe,
} from "@tanstack/query-persist-client-core";
import type { QueryClient } from "@tanstack/query-core";
import { type JSX, Storage, cleanup, signal } from "nsh";
import { IsRestoringProvider } from "./isRestoring";
import { QueryClientProvider } from "./QueryClientProvider";

/** Options of `createStoragePersister`: those of `createAsyncStoragePersister`, minus its storage. */
export type StoragePersisterOptions = Omit<Parameters<typeof createAsyncStoragePersister>[0], "storage">;

/**
 * TanStack's async-storage persister over NSH's `Storage`. The whole client is saved as one value under
 * `key` (default `"nsh-query-client"`), at most once per `throttleTime` (default 1000 ms). `Storage` holds
 * 5 MB; a larger save fails and `retry` decides whether to try again.
 */
export function createStoragePersister(options: StoragePersisterOptions = {}) {
  return createAsyncStoragePersister({ key: "nsh-query-client", ...options, storage: Storage });
}

/** Props of `PersistQueryClientProvider`. */
export interface PersistQueryClientProviderProps {
  client: QueryClient;
  /** The persister and the options of `persistQueryClientRestore` and `persistQueryClientSave`. */
  persistOptions: Omit<PersistQueryClientOptions, "queryClient">;
  /** Runs after the client is restored. */
  onSuccess?: () => unknown;
  /** Runs when restoring failed, after the persisted copy was removed. */
  onError?: () => unknown;
  children?: JSX.Element;
}

/**
 * Restores `client` from its persister before its queries run, then saves it on every cache change.
 * Queries wait, through `useIsRestoring`, until the restore has finished.
 */
export function PersistQueryClientProvider(props: PersistQueryClientProviderProps): JSX.Element {
  const restoring = signal(true);
  const options = { ...props.persistOptions, queryClient: props.client };
  let stopped = false;
  let unsubscribe: (() => void) | undefined;

  persistQueryClientRestore(options)
    .then(() => props.onSuccess?.())
    .catch(() => props.onError?.())
    .finally(() => {
      restoring.set(false);
      if (!stopped) unsubscribe = persistQueryClientSubscribe(options);
    });

  cleanup(() => {
    stopped = true;
    unsubscribe?.();
  });

  return QueryClientProvider({
    client: props.client,
    get children() {
      return IsRestoringProvider({
        value: restoring,
        get children() {
          return props.children;
        },
      });
    },
  });
}
