import { expect, test } from "nsh/test";
import { API, show } from "./support";

test("two observers of one key share one fetch", async () => {
  const app = await show("shared", { network: { [`GET ${API}/shared`]: { body: { n: 1 } } } });
  await app.waitFor("text:shared 1 >> nth=1");
  expect(app.requests().length).toBe(1);
});

test("a fresh entry skips the fetch, and an expired staleTime refetches on mount", async () => {
  let n = 0;
  const app = await show("cached", { network: { [`GET ${API}/count`]: () => ({ body: { n: ++n } }) } });
  await app.waitFor("text:count 1");
  await app.press("id:toggle");
  await app.press("id:toggle");
  await app.waitFor("text:count 1");
  expect(app.requests().length).toBe(1);

  await app.press("id:toggle");
  await app.clock!.runFor(61_000);
  await app.press("id:toggle");
  await app.waitFor("text:count 2");
  expect(app.requests().length).toBe(2);
});

test("gcTime drops an unobserved entry", async () => {
  let n = 0;
  const app = await show("gc", {
    network: { [`GET ${API}/count`]: () => ({ body: { n: ++n }, delay: n === 1 ? 0 : 500 }) },
  });
  await app.waitFor("text:count 1");
  await app.press("id:toggle");
  await app.clock!.runFor(1500);
  await app.press("id:toggle");
  expect(app.shows("text:count none")).toBe(true);
  await app.waitFor("text:count 2");
  expect(app.requests().length).toBe(2);
});

test("retries count in failureCount while status stays pending, then succeed", async () => {
  let attempts = 0;
  const app = await show("retry", {
    network: { [`GET ${API}/flaky`]: () => (++attempts < 3 ? { status: 500 } : { body: { ok: true } }) },
  });
  await app.waitFor("text:failures 1");
  expect(app.shows("text:status pending")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:failures 2");
  expect(app.shows("text:status pending")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:data true");
  expect(app.shows("text:status success")).toBe(true);
  expect(app.shows("text:failures 0")).toBe(true);
  expect(attempts).toBe(3);
});

test("a failure after the last retry gives status error and failureReason", async () => {
  const app = await show("fails", { network: { [`GET ${API}/flaky`]: { status: 500 } } });
  await app.clock!.runFor(2000);
  await app.waitFor("text:status error");
  expect(app.shows("text:reason status 500")).toBe(true);
  expect(app.requests().length).toBe(3);
});

test("retry and retryDelay as functions are honored", async () => {
  const app = await show("retryfns", { network: { [`GET ${API}/flaky`]: { status: 500 } } });
  // retryDelay(0) is 100 ms, then retryDelay(1) is 200 ms; retry(2) stops after three attempts.
  expect(app.requests().length).toBe(1);
  await app.clock!.runFor(99);
  expect(app.requests().length).toBe(1);
  await app.clock!.runFor(1);
  expect(app.requests().length).toBe(2);
  await app.clock!.runFor(199);
  expect(app.requests().length).toBe(2);
  await app.clock!.runFor(1);
  expect(app.requests().length).toBe(3);
  await app.waitFor("text:status error");
});

test("invalidating reloads what is observed and replaces a load in flight, dropping its late answer", async () => {
  let n = 0;
  const app = await show("invalidate", {
    network: { [`GET ${API}/count`]: () => { n += 1; return { body: { n }, delay: n === 2 ? 1000 : 10 }; } },
  });
  await app.waitFor("text:count 1");
  await app.press("id:slow");
  expect(app.requests().length).toBe(2);
  await app.press("id:invalidate");
  await app.waitFor("text:count 3");
  await app.clock!.runFor(2000);
  expect(app.shows("text:count 3")).toBe(true);
  expect(app.shows("text:count 2")).toBe(false);
  expect(app.requests().length).toBe(3);
});

test("an AbortSignal passed to fetch fires when a query is cancelled", async () => {
  const app = await show("abort", { network: { [`GET ${API}/abort`]: { body: { n: 1 }, delay: 1000 } } });
  expect(app.shows("text:aborted false")).toBe(true);
  await app.press("id:cancel");
  await app.waitFor("text:aborted true");
});

test("a tracked key moves to the new entry and back to its cache", async () => {
  const app = await show("bykey", {
    network: {
      [`GET ${API}/users/1`]: { body: { name: "Ada" } },
      [`GET ${API}/users/2`]: { body: { name: "Lin" } },
    },
  });
  await app.waitFor("text:user Ada");
  await app.press("id:two");
  await app.waitFor("text:user Lin");
  await app.press("id:one");
  expect(app.shows("text:user Ada")).toBe(true);
  expect(app.requests().length).toBe(2);
});

test("enabled as a tracked value, and skipToken", async () => {
  const app = await show("gated", {
    network: {
      [`GET ${API}/gated`]: { body: { v: "ran" } },
      [`GET ${API}/skipped`]: { body: { v: "ran" } },
    },
  });
  expect(app.shows("text:gated fetch idle")).toBe(true);
  expect(app.shows("text:gated none")).toBe(true);
  expect(app.shows("text:skipped none")).toBe(true);
  expect(app.requests().length).toBe(0);
  await app.press("id:enable");
  await app.waitFor("text:gated ran");
  await app.waitFor("text:skipped ran");
  expect(app.requests().length).toBe(2);
});

test("select and keepPreviousData, with isPlaceholderData", async () => {
  const app = await show("select", {
    network: {
      [`GET ${API}/page/1`]: { body: { title: "alpha" } },
      [`GET ${API}/page/2`]: { body: { title: "bravo" }, delay: 1000 },
    },
  });
  await app.waitFor("text:title ALPHA");
  await app.press("id:next");
  expect(app.shows("text:title ALPHA")).toBe(true);
  expect(app.shows("text:placeholder true")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:title BRAVO");
  expect(app.shows("text:placeholder false")).toBe(true);
});

test("initialData, initialDataUpdatedAt and placeholderData", async () => {
  const app = await show("seeded", {
    network: {
      [`GET ${API}/old`]: { body: { v: "network" } },
      [`GET ${API}/slow`]: { body: { v: "loaded" }, delay: 1000 },
    },
  });
  expect(app.shows("text:seeded seed")).toBe(true);
  expect(app.requests().map((request) => request.url)).not.toContain(`${API}/seeded`);
  await app.waitFor("text:old network");
  expect(app.shows("text:slow placeholder true")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:slow loaded");
});

test("refetchInterval, and it stops in the background unless refetchIntervalInBackground", async () => {
  let poll = 0;
  let background = 0;
  const app = await show("interval", {
    network: {
      [`GET ${API}/poll`]: () => ({ body: { n: ++poll } }),
      [`GET ${API}/pollbg`]: () => ({ body: { n: ++background } }),
    },
  });
  await app.waitFor("text:poll 1");
  await app.clock!.runFor(1000);
  await app.waitFor("text:poll 2");
  expect(poll).toBe(2);

  await app.setLifecycle("background");
  await app.clock!.runFor(5000);
  expect(poll).toBe(2);
  expect(background).toBeGreaterThan(2);
});

test("13b. an inactive app is still visible, so its refetchInterval keeps running", async () => {
  let poll = 0;
  let background = 0;
  const app = await show("interval", {
    network: {
      [`GET ${API}/poll`]: () => ({ body: { n: ++poll } }),
      [`GET ${API}/pollbg`]: () => ({ body: { n: ++background } }),
    },
  });
  await app.waitFor("text:poll 1");
  await app.setLifecycle("inactive");
  await app.clock!.runFor(1000);
  await app.waitFor("text:poll 2");
  expect(poll).toBe(2);
});

test("refetchOnWindowFocus on the lifecycle change, using app.setLifecycle", async () => {
  let focused = 0;
  let unfocused = 0;
  const app = await show("focus", {
    network: {
      [`GET ${API}/focus`]: () => ({ body: { n: ++focused } }),
      [`GET ${API}/nofocus`]: () => ({ body: { n: ++unfocused } }),
    },
  });
  await app.waitFor("text:focus 1");
  await app.setLifecycle("background");
  await app.setLifecycle("active");
  await app.waitFor("text:focus 2");
  expect(app.shows("text:nofocus 1")).toBe(true);
  expect(unfocused).toBe(1);
});

test("onlineManager.setOnline(false) pauses, and setting it back resumes", async () => {
  let n = 0;
  const app = await show("online", { network: { [`GET ${API}/online`]: () => ({ body: { n: ++n } }) } });
  await app.waitFor("text:online 1");
  await app.press("id:offline");
  await app.press("id:refetch");
  await app.waitFor("text:fetch paused");
  expect(app.requests().length).toBe(1);
  await app.press("id:online");
  await app.waitFor("text:online 2");
  expect(app.requests().length).toBe(2);
});

test("a binding on query.data does not rerun when only fetchStatus changes", async () => {
  let v = "v1";
  const app = await show("renders", { network: { [`GET ${API}/watch`]: () => ({ body: { v }, delay: 1000 }) } });
  await app.clock!.runFor(1000);
  await app.waitFor("text:v1");
  const rendersLine = () => app.texts().find((line) => line.startsWith("renders "));
  const before = rendersLine();

  await app.press("id:refetch");
  await app.waitFor("text:fetch fetching");
  await app.clock!.runFor(1000);
  await app.waitFor("text:fetch idle");
  expect(rendersLine()).toBe(before);

  v = "v2";
  await app.press("id:refetch");
  await app.clock!.runFor(1000);
  await app.waitFor("text:v2");
  expect(rendersLine()).not.toBe(before);
});

test("pages load in both directions, stop at the ends, honor maxPages, and refetch every page", async () => {
  const app = await show("pages", {
    network: {
      [`GET ${API}/pages/1`]: { body: { number: 1, next: 2, previous: null } },
      [`GET ${API}/pages/2`]: { body: { number: 2, next: 3, previous: 1 } },
      [`GET ${API}/pages/3`]: { body: { number: 3, next: null, previous: 2 } },
    },
  });
  await app.waitFor("text:pages 2");
  expect(app.shows("text:hasNext true")).toBe(true);
  expect(app.shows("text:hasPrev true")).toBe(true);

  await app.press("id:next");
  await app.waitFor("text:pages 2,3");
  expect(app.shows("text:hasNext false")).toBe(true);

  await app.press("id:previous");
  await app.waitFor("text:pages 1,2");
  expect(app.shows("text:params 1,2")).toBe(true);
  expect(app.shows("text:hasPrev false")).toBe(true);

  await app.press("id:refetch");
  await app.waitFor("text:pages 1,2");
  expect(app.requests().length).toBe(5);
});
