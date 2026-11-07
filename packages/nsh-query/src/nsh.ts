import { environmentManager, notifyManager } from "@tanstack/query-core";
import { batch } from "nsh";

// NSH has no `window`, so query-core would take it for a server: default retries of 0, an infinite
// gcTime and no timers. An NSH app is a client, so it keeps TanStack's client defaults.
environmentManager.setIsServer(() => false);

// Settings, not subscriptions: one batch per change, for the whole app. Focus is set up by
// `QueryClientProvider`, so its effect has an owner.
notifyManager.setBatchNotifyFunction(batch);
