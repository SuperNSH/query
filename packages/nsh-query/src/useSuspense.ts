import { NshQueryError, errorCodes } from "./errors";

const message = (name: string): string =>
  `nsh-query: ${name} is not available. NSH 0.4.0 has no Suspense boundary, so a component cannot wait for data by suspending. Use useQuery and read isPending, or the NSH resource() for async values.`;

/**
 * Not available on NSH: it has no Suspense boundary. Calling it throws `ERR_NSH_QUERY_NO_SUSPENSE`.
 * @throws {NshQueryError} Always.
 */
export function useSuspenseQuery(): never {
  throw new NshQueryError(errorCodes.noSuspense, message("useSuspenseQuery"));
}

/**
 * Not available on NSH: it has no Suspense boundary. Calling it throws `ERR_NSH_QUERY_NO_SUSPENSE`.
 * @throws {NshQueryError} Always.
 */
export function useSuspenseInfiniteQuery(): never {
  throw new NshQueryError(errorCodes.noSuspense, message("useSuspenseInfiniteQuery"));
}

/**
 * Not available on NSH: it has no Suspense boundary. Calling it throws `ERR_NSH_QUERY_NO_SUSPENSE`.
 * @throws {NshQueryError} Always.
 */
export function useSuspenseQueries(): never {
  throw new NshQueryError(errorCodes.noSuspense, message("useSuspenseQueries"));
}
