import { focusManager } from "@tanstack/query-core";
import { effect, lifecycle, untracked } from "nsh";

/**
 * Makes focus follow the app's lifecycle while a `QueryClientProvider` is shown. The effect is made
 * inside the provider, so it ends with it. Visible but unfocused ("inactive") still counts as
 * focused, as query-core's own check treats only a hidden page as unfocused.
 */
export function followLifecycle(): void {
  focusManager.setEventListener((setFocused) => {
    setFocused(lifecycle() !== "background");
    effect(() => {
      const focused = lifecycle() !== "background";
      untracked(() => setFocused(focused));
    });
    return undefined;
  });
}

/** Stops following the lifecycle. Called when the provider that started it goes. */
export function releaseLifecycle(): void {
  focusManager.setEventListener(() => undefined);
}
