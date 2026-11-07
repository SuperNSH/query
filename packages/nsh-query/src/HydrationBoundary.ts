import { type DehydratedState, type HydrateOptions, hydrate } from "@tanstack/query-core";
import { type JSX, effect, untracked } from "nsh";
import { resolveQueryClient } from "./QueryClientProvider";

/** Props of `HydrationBoundary`. */
export interface HydrationBoundaryProps {
  /** A state from `dehydrate` (on a server, or from another client). Applied before the children render. */
  state?: DehydratedState | null;
  /** The options of `hydrate`, such as `defaultOptions`. */
  options?: HydrateOptions;
  children?: JSX.Element;
}

/**
 * Hydrates the nearest client with `state` before its children render, so their queries start from the
 * dehydrated data. A later change of `state` is hydrated too.
 */
export function HydrationBoundary(props: HydrationBoundaryProps): JSX.Element {
  const client = resolveQueryClient();
  const apply = (state: DehydratedState | null | undefined) => {
    if (state) hydrate(client, state, untracked(() => props.options));
  };
  apply(untracked(() => props.state));
  let applied = untracked(() => props.state);
  effect(() => {
    const next = props.state;
    untracked(() => {
      if (next === applied) return;
      applied = next;
      apply(next);
    });
  });
  return props.children as JSX.Element;
}
