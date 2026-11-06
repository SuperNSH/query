import { expect, open, test } from "nsh/test";
import type { App } from "nsh/test";

const API = "https://api.example.com";

type Network = NonNullable<NonNullable<Parameters<typeof open>[1]>["network"]>;

async function show(name: string, network: Network): Promise<App> {
  const app = await open("../App.tsx", { clock: {}, network });
  await app.press(`id:open-${name}`);
  return app;
}

test("two observers of one key share one load", async () => {
  const app = await show("Cached", { [`GET ${API}/count`]: { body: { n: 1 } } });
  await app.waitFor("text:count 1 >> nth=1");
  expect(app.requests().length).toBe(1);
});

test("a fresh entry skips the load and an expired one reloads", async () => {
  let n = 0;
  const app = await show("Cached", { [`GET ${API}/count`]: () => ({ body: { n: ++n } }) });
  await app.waitFor("text:count 1 >> nth=0");
  await app.press("id:toggle");
  await app.press("id:toggle");
  await app.waitFor("text:count 1 >> nth=0");
  expect(app.requests().length).toBe(1);

  await app.press("id:toggle");
  await app.clock!.runFor(61_000);
  await app.press("id:toggle");
  await app.waitFor("text:count 2 >> nth=0");
  expect(app.requests().length).toBe(2);
});

test("invalidating reloads what is observed", async () => {
  let n = 0;
  const app = await show("Cached", { [`GET ${API}/count`]: () => ({ body: { n: ++n } }) });
  await app.waitFor("text:count 1 >> nth=0");
  await app.press("id:invalidate");
  await app.waitFor("text:count 2 >> nth=0");
});

test("invalidating replaces a load in flight and drops its late answer", async () => {
  let n = 0;
  const app = await show("Cached", {
    [`GET ${API}/count`]: () => {
      n += 1;
      return { body: { n }, delay: n === 1 ? 1000 : 10 };
    },
  });
  await app.press("id:invalidate");
  await app.clock!.runFor(20);
  await app.waitFor("text:count 2 >> nth=0");
  await app.clock!.runFor(2000);
  expect(app.count("text:count 2")).toBe(2);
});

test("retries stay loading and end in success", async () => {
  let attempts = 0;
  const app = await show("Flaky", {
    [`GET ${API}/flaky`]: () => (++attempts < 3 ? { status: 500 } : { body: { ok: true } }),
  });
  await app.waitFor("text:retries 1");
  expect(app.shows("text:failed false")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:retries 2");
  await app.clock!.runFor(1000);
  await app.waitFor("text:flaky true");
  expect(app.shows("text:failed false")).toBe(true);
  expect(app.shows("text:retries 0")).toBe(true);
});

test("a failure after the last retry reports its error", async () => {
  const app = await show("Flaky", { [`GET ${API}/flaky`]: { status: 500 } });
  await app.clock!.runFor(1000);
  await app.clock!.runFor(1000);
  await app.waitFor("text:failed true");
  expect(app.shows("text:error Error: status 500")).toBe(true);
});

test("a tracked key moves to the new entry and back to its cache", async () => {
  const app = await show("ByKey", {
    [`GET ${API}/users/1`]: { body: { name: "Ada" } },
    [`GET ${API}/users/2`]: { body: { name: "Lin" } },
  });
  await app.waitFor("text:user Ada");
  await app.press("id:two");
  await app.waitFor("text:user Lin");
  await app.press("id:one");
  await app.waitFor("text:user Ada");
  expect(app.requests().length).toBe(2);
});

test("a disabled query waits for its condition", async () => {
  const app = await show("Gated", { [`GET ${API}/gated`]: { body: { v: "ran" } } });
  expect(app.shows("text:gated idle")).toBe(true);
  expect(app.requests().length).toBe(0);
  await app.press("id:enable");
  await app.waitFor("text:gated ran");
});

test("seeded data counts as fresh", async () => {
  const app = await show("Seeded", { [`GET ${API}/seeded`]: { body: { v: "from network" } } });
  await app.waitFor("text:seeded from cache");
  expect(app.requests().length).toBe(0);
});

test("a successful write invalidates and the query loads the new value", async () => {
  let stored = 5;
  const app = await show("Save", {
    [`GET ${API}/server`]: () => ({ body: { n: stored } }),
    [`POST ${API}/server`]: (request) => {
      stored = JSON.parse(request.body!).n;
      return { body: { n: stored }, delay: 100 };
    },
  });
  await app.waitFor("text:server 5");
  await app.press("id:save");
  expect(app.shows("text:saving true")).toBe(true);
  await app.clock!.runFor(150);
  await app.waitFor("text:server 9");
  expect(app.shows("text:saving false")).toBe(true);
});

test("an optimistic write rolls back after the last attempt fails", async () => {
  const app = await show("Optimistic", {});
  await app.waitFor("text:liked false");
  await app.press("id:like");
  await app.waitFor("text:liked true");
  await app.clock!.runFor(200);
  await app.waitFor("text:liked false");
  await app.waitFor("text:like error yes");
});

test("pages load in both directions and stop at the ends", async () => {
  const route = (number: number) => ({
    body: { number, next: number < 3 ? number + 1 : null, previous: number > 1 ? number - 1 : null },
    delay: 10,
  });
  const app = await show("Pages", {
    [`GET ${API}/pages/1`]: route(1),
    [`GET ${API}/pages/2`]: route(2),
    [`GET ${API}/pages/3`]: route(3),
  });
  await app.clock!.runFor(20);
  await app.waitFor("text:pages 2");
  expect(app.shows("text:more true")).toBe(true);
  expect(app.shows("text:earlier true")).toBe(true);

  await app.press("id:next");
  await app.press("id:next");
  expect(app.requests().length).toBe(2);
  await app.clock!.runFor(20);
  await app.waitFor("text:pages 2,3");
  expect(app.shows("text:more false")).toBe(true);

  await app.press("id:previous");
  await app.clock!.runFor(20);
  await app.waitFor("text:pages 1,2,3");
  expect(app.shows("text:earlier false")).toBe(true);

  await app.press("id:refetch");
  await app.clock!.runFor(20);
  await app.waitFor("text:pages 2");
});
