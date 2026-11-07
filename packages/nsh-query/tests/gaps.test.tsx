import { expect, test } from "nsh/test";
import { API, show } from "./support";

test("unmount: a provider's client stops following focus when the provider goes", async () => {
  let n = 0;
  const app = await show("unmount", { network: { [`GET ${API}/unmount`]: () => ({ body: { n: ++n } }) } });
  await app.waitFor("text:mounted true");
  expect(n).toBe(1);
  await app.setLifecycle("background");
  await app.setLifecycle("active");
  await app.waitUntil(() => n === 2);

  await app.press("id:toggle");
  await app.setLifecycle("background");
  await app.setLifecycle("active");
  await app.clock!.runFor(100);
  expect(n).toBe(2);
});

test("experimental_streamedQuery collects the chunks of a stream", async () => {
  const app = await show("streamed", {});
  await app.waitFor("text:stream 1,2");
});

test("networkMode always and offlineFirst run their first attempt offline; online waits", async () => {
  let always = 0;
  let first = 0;
  let online = 0;
  const app = await show("network", {
    network: {
      [`GET ${API}/always`]: () => ({ body: { n: ++always } }),
      [`GET ${API}/first`]: () => ({ body: { n: ++first } }),
      [`GET ${API}/online-mode`]: () => ({ body: { n: ++online } }),
    },
  });
  await app.waitFor("text:online 1");
  await app.press("id:offline");
  await app.press("id:refetch-all");
  await app.waitFor("text:always 2");
  await app.waitFor("text:first 2");
  expect(app.shows("text:online fetch paused")).toBe(true);
  expect(online).toBe(1);
  await app.press("id:online");
  await app.waitFor("text:online 2");
});

test("refetchOnMount false keeps stale data on remount; the default refetches", async () => {
  let kept = 0;
  let refetched = 0;
  const app = await show("mount", {
    network: {
      [`GET ${API}/mount-no`]: () => ({ body: { n: ++kept } }),
      [`GET ${API}/mount-yes`]: () => ({ body: { n: ++refetched } }),
    },
  });
  await app.waitFor("text:refetched 1");
  await app.press("id:toggle");
  await app.press("id:toggle");
  await app.waitFor("text:refetched 2");
  expect(kept).toBe(1);
});

test("refetchOnReconnect refetches a stale query on reconnect unless turned off", async () => {
  let on = 0;
  let off = 0;
  const app = await show("reconnect", {
    network: {
      [`GET ${API}/re-on`]: () => ({ body: { n: ++on } }),
      [`GET ${API}/re-off`]: () => ({ body: { n: ++off } }),
    },
  });
  await app.waitFor("text:re-on 1");
  await app.waitFor("text:re-off 1");
  await app.press("id:offline");
  await app.press("id:online");
  await app.waitFor("text:re-on 2");
  expect(off).toBe(1);
});

test("notifyOnChangeProps: a fetch that changes only fetchStatus does not notify the screen", async () => {
  const app = await show("notify", { network: { [`GET ${API}/notify`]: { body: { n: 1 }, delay: 1000 } } });
  await app.clock!.runFor(1000);
  await app.waitFor("text:notify 1");
  expect(app.shows("text:notify fetch idle")).toBe(true);
  await app.press("id:refetch");
  // The request is in flight, but only data is notified, so the screen still shows idle.
  expect(app.shows("text:notify fetch idle")).toBe(true);
  await app.clock!.runFor(1000);
  expect(app.shows("text:notify fetch idle")).toBe(true);
});

test("meta reaches the queryFn", async () => {
  const app = await show("meta", {});
  await app.waitFor("text:meta from meta");
});

test("suspense: true throws ERR_NSH_QUERY_NO_SUSPENSE to the nearest boundary", async () => {
  const app = await show("suspense", {});
  await app.waitFor("text:suspense ERR_NSH_QUERY_NO_SUSPENSE");
});

test("useSuspenseQuery, useSuspenseInfiniteQuery and useSuspenseQueries throw the same coded error", async () => {
  const app = await show("suspense-hooks", {});
  await app.waitFor("text:hook a ERR_NSH_QUERY_NO_SUSPENSE");
  expect(app.shows("text:hook b ERR_NSH_QUERY_NO_SUSPENSE")).toBe(true);
  expect(app.shows("text:hook c ERR_NSH_QUERY_NO_SUSPENSE")).toBe(true);
});

test("options as a plain object, and the client as a plain QueryClient", async () => {
  const plain = await show("plain", {});
  await plain.waitFor("text:plain ok");
  const withClient = await show("plain-client", {});
  await withClient.waitFor("text:plain client 2");
});

test("useIsFetching and useIsMutating take a filter accessor", async () => {
  const app = await show("filter-accessor", {
    network: {
      [`GET ${API}/af-a`]: { body: {}, delay: 1000 },
      [`POST ${API}/af-m`]: { body: {}, delay: 1000 },
    },
  });
  await app.waitFor("text:fetching 1");
  await app.press("id:mutate");
  await app.waitFor("text:mutating 1");
  await app.press("id:switch");
  expect(app.shows("text:fetching 0")).toBe(true);
  expect(app.shows("text:mutating 0")).toBe(true);
});

test("throwOnError reaches the boundary, and a reset of the boundary retries the query", async () => {
  let attempts = 0;
  const app = await show("reset", {
    network: { [`GET ${API}/reset`]: () => (++attempts === 1 ? { status: 500 } : { body: { n: 2 } }) },
  });
  await app.waitFor("text:reset failed status 500");
  await app.press("id:retry");
  await app.waitFor("text:reset 2");
  expect(attempts).toBe(2);
});

test("usePrefetchQuery and usePrefetchInfiniteQuery start the fetch before the query reads it", async () => {
  let calls = 0;
  const app = await show("prefetch", {
    network: { [`GET ${API}/pre`]: () => ({ body: { n: ++calls } }) },
  });
  await app.waitFor("text:prefetched 1");
  await app.waitFor("text:prefetched inf 1");
  expect(app.requests().length).toBe(1);
});

test("HydrationBoundary applies dehydrated state before its children read it", async () => {
  const app = await show("hydrate", {});
  await app.waitFor("text:hydrated from server");
  expect(app.requests().length).toBe(0);
});

test("after a reset, a query that fails again shows its error as state, without throwing it to the boundary", async () => {
  const app = await show("reset", {
    network: { [`GET ${API}/reset`]: { status: 500 } },
  });
  await app.waitFor("text:reset failed status 500");
  await app.press("id:retry");
  await app.waitFor("text:reset error true");
  expect(app.shows("text:reset failed status 500")).toBe(false);
});
