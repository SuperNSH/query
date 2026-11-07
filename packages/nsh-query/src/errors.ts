/**
 * An error the adapter raises itself. `code` is stable across releases; match on it, not on `message`.
 */
export class NshQueryError extends Error {
  /** A stable identifier such as `ERR_NSH_QUERY_NO_CLIENT`. */
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "NshQueryError";
    this.code = code;
  }
}

/** The codes `NshQueryError` carries. */
export const errorCodes = {
  noClient: "ERR_NSH_QUERY_NO_CLIENT",
  noSuspense: "ERR_NSH_QUERY_NO_SUSPENSE",
} as const;
