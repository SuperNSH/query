import { context, type JSX, signal } from "nsh";

/** What `useQueryErrorResetBoundary` gives: whether a reset was asked for, and the means to ask. */
export interface QueryErrorResetValue {
  /** Whether a query's error is no longer thrown to the error boundary, because `reset` was called. */
  isReset(): boolean;
  /** Stops query errors from being thrown to the boundary, until `clearReset`. */
  reset(): void;
  /** Lets query errors be thrown to the boundary again. */
  clearReset(): void;
}

const noBoundary: QueryErrorResetValue = {
  isReset: () => false,
  reset: () => undefined,
  clearReset: () => undefined,
};

const ErrorResetContext = context<QueryErrorResetValue>(noBoundary);

/** Props of `QueryErrorResetBoundary`. */
export interface QueryErrorResetBoundaryProps {
  children?: JSX.Element;
}

/**
 * Lets a reset of an error boundary also reset the queries under it. Call `reset` in the boundary's
 * fallback, then the boundary's own `reset`: the children render again and their queries fetch
 * after the failure instead of throwing it again.
 */
export function QueryErrorResetBoundary(props: QueryErrorResetBoundaryProps): JSX.Element {
  const resetting = signal(false);
  const value: QueryErrorResetValue = {
    isReset: () => resetting(),
    reset: () => resetting.set(true),
    clearReset: () => resetting.set(false),
  };
  return ErrorResetContext.Provider({
    value,
    get children() {
      return props.children;
    },
  });
}

/** The nearest `QueryErrorResetBoundary`'s value, or a no-op one when there is none above. */
export function useQueryErrorResetBoundary(): QueryErrorResetValue {
  return ErrorResetContext();
}
