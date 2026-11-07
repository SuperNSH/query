import { open } from "nsh/test";
import type { App, ClockOptions, OpenOptions } from "nsh/test";

export const API = "https://api.example.com";

/**
 * Opens the app on a manual clock and shows `screen` from its home list. The viewport is taller than
 * a phone so that a screen's controls and log lines all stay on screen, where a step can reach them.
 */
export async function show(
  screen: string,
  options: Pick<OpenOptions, "network" | "storage"> & { clock?: ClockOptions } = {},
): Promise<App> {
  const app = await open("../App.tsx", { height: 3000, ...options, clock: options.clock ?? {} });
  await app.press(`id:open-${screen}`);
  return app;
}
