import type { QueryClient } from "@tanstack/query-core";
import { type Accessor, cleanup, context, type JSX } from "nsh";
import { NshQueryError, errorCodes } from "./errors";
import { followLifecycle, releaseLifecycle } from "./focus";

const QueryClientContext = context<Accessor<QueryClient> | undefined>(undefined);

/**
 * The client a hook uses: the one passed in (a client, or an accessor of one), else the nearest
 * `QueryClientProvider`'s. Call it in a component body: the client is read once.
 * @throws {NshQueryError} `ERR_NSH_QUERY_NO_CLIENT` when neither exists.
 */
export function resolveQueryClient(queryClient?: QueryClient | Accessor<QueryClient | undefined>): QueryClient {
  const given = typeof queryClient === "function" ? queryClient() : queryClient;
  if (given) return given;
  const provided = QueryClientContext();
  if (!provided) {
    throw new NshQueryError(errorCodes.noClient, "No QueryClient set, use QueryClientProvider to set one");
  }
  return provided();
}

/**
 * The `QueryClient` of the nearest provider, or the one passed in.
 * @throws {NshQueryError} `ERR_NSH_QUERY_NO_CLIENT` when neither exists.
 */
export function useQueryClient(queryClient?: QueryClient): QueryClient {
  return resolveQueryClient(queryClient);
}

/** Props of `QueryClientProvider`. */
export interface QueryClientProviderProps {
  /** The client to provide. It is mounted while this component is shown. */
  client: QueryClient;
  children?: JSX.Element;
}

/**
 * Provides `client` to the components below. While shown, the client is mounted and focus follows the
 * app's lifecycle; both end when this component goes.
 */
export function QueryClientProvider(props: QueryClientProviderProps): JSX.Element {
  props.client.mount();
  followLifecycle();
  cleanup(() => {
    releaseLifecycle();
    props.client.unmount();
  });
  // `children` is a getter so the children render after the context is set, as JSX children do.
  return QueryClientContext.Provider({
    value: () => props.client,
    get children() {
      return props.children;
    },
  });
}
