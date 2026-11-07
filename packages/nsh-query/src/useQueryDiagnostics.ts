import type { Mutation, Query, QueryClient } from "@tanstack/query-core";
import { replaceEqualDeep } from "@tanstack/query-core";
import { type Accessor, cleanup, signal, untracked } from "nsh";
import { resolveQueryClient } from "./QueryClientProvider";

/** One query, as `useQueryDiagnostics` reports it. */
export interface QueryDiagnosticEntry {
  queryHash: string;
  queryKey: readonly unknown[];
  status: string;
  fetchStatus: string;
  /** How many observers are subscribed to the query now. */
  observers: number;
  /** Whether the query's data is stale now. */
  stale: boolean;
  /** When the data last changed, in milliseconds since the epoch (0 when it never has). */
  dataUpdatedAt: number;
  /** How many times the query has failed in its current fetch. */
  failureCount: number;
}

/** One mutation, as `useQueryDiagnostics` reports it. */
export interface MutationDiagnosticEntry {
  mutationId: number;
  mutationKey: readonly unknown[] | undefined;
  status: string;
  /** How many times the mutation has failed in its current run. */
  failureCount: number;
}

/** A snapshot of the client's queries and mutations. */
export interface QueryDiagnostics {
  queries: QueryDiagnosticEntry[];
  mutations: MutationDiagnosticEntry[];
}

const snapshot = (client: QueryClient): QueryDiagnostics => ({
  queries: client.getQueryCache().getAll().map((query: Query) => ({
    queryHash: query.queryHash,
    queryKey: query.queryKey,
    status: query.state.status,
    fetchStatus: query.state.fetchStatus,
    observers: query.getObserversCount(),
    stale: query.isStale(),
    dataUpdatedAt: query.state.dataUpdatedAt,
    failureCount: query.state.fetchFailureCount,
  })),
  mutations: client.getMutationCache().getAll().map((mutation: Mutation) => ({
    mutationId: mutation.mutationId,
    mutationKey: mutation.options.mutationKey,
    status: mutation.state.status,
    failureCount: mutation.state.failureCount,
  })),
});

/**
 * The queries and mutations of the client, as an accessor. It is read in a binding, and it changes
 * only when a value in the snapshot changes: a cache event that changes nothing wakes nothing. Use it
 * for a debug panel; the snapshot is plain data, so it can be logged or shown as it is.
 */
export function useQueryDiagnostics(queryClient?: QueryClient | Accessor<QueryClient | undefined>): Accessor<QueryDiagnostics> {
  const client = resolveQueryClient(queryClient);
  const current = signal(untracked(() => snapshot(client)));

  const refresh = () => {
    const next = replaceEqualDeep(untracked(current), snapshot(client));
    if (next !== untracked(current)) current.set(next);
  };
  const stopQueries = client.getQueryCache().subscribe(refresh);
  const stopMutations = client.getMutationCache().subscribe(refresh);
  cleanup(() => {
    stopQueries();
    stopMutations();
  });

  return current;
}
