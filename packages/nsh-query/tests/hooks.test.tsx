import { expect, test } from "nsh/test";
import { API, show } from "./support";

test("useQueries with combine, useIsFetching, useIsMutating, useMutationState", async () => {
  const app = await show("many", {
    network: {
      [`GET ${API}/a`]: { body: { n: 1 } },
      [`GET ${API}/b`]: { body: { n: 2 }, delay: 1000 },
      [`POST ${API}/note`]: { body: {}, delay: 1000 },
    },
  });
  await app.waitFor("text:total 1");
  expect(app.shows("text:pending 1")).toBe(true);
  expect(app.shows("text:fetching 1")).toBe(true);

  await app.clock!.runFor(1000);
  await app.waitFor("text:total 3");
  expect(app.shows("text:pending 0")).toBe(true);
  expect(app.shows("text:fetching 0")).toBe(true);

  await app.press("id:note-a");
  await app.press("id:note-b");
  expect(app.shows("text:mutating 2")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:mutating 0");
  expect(app.shows("text:vars a,b")).toBe(true);
});

test("setQueryData with an updater, getQueryData, getQueryState, prefetchQuery, fetchQuery, ensureQueryData, removeQueries, resetQueries, cancelQueries, setQueryDefaults", async () => {
  const app = await show("client", { network: { [`GET ${API}/slowc`]: { body: { n: 1 }, delay: 1000 } } });

  await app.press("id:set");
  expect(app.texts()).toContain("set 11");
  await app.press("id:get");
  expect(app.texts()).toContain("get 11");
  await app.press("id:state");
  expect(app.texts()).toContain("state success");
  await app.press("id:prefetch");
  expect(app.texts()).toContain("prefetch prefetched");
  await app.press("id:fetch");
  expect(app.texts()).toContain("fetch fetched");
  await app.press("id:ensure");
  expect(app.texts()).toContain("ensure 11");
  await app.press("id:remove");
  expect(app.texts()).toContain("removed true");
  await app.press("id:reset");
  expect(app.texts()).toContain("reset true");

  await app.press("id:start-slow");
  await app.press("id:cancel");
  expect(app.texts()).toContain("cancelled idle");
  await app.press("id:fetching");
  expect(app.texts()).toContain("fetching 0");

  await app.press("id:defaults");
  expect(app.texts()).toContain("defaults 5000");
  await app.press("id:mutation-defaults");
  expect(app.texts()).toContain("mutation defaults 1");
  await app.press("id:default-options");
  expect(app.texts()).toContain("default options 9000");

  await app.press("id:set-many");
  expect(app.texts()).toContain("same client true");
  expect(app.texts()).toContain("set many 6");
  await app.press("id:get-many");
  expect(app.texts()).toContain("get many 1");
  await app.press("id:refetch-none");
  expect(app.texts()).toContain("refetch none true");
  await app.press("id:mutating");
  expect(app.texts()).toContain("mutating 0");
  await app.press("id:resume");
  expect(app.texts()).toContain("resumed done");
  await app.press("id:infinite");
  expect(app.texts()).toContain("infinite 1");
  await app.press("id:fetch-infinite");
  expect(app.texts()).toContain("fetch infinite 3");
  await app.press("id:infinite-query");
  expect(app.texts()).toContain("infinite query 5");
});

test("query filters: exact, type, stale, predicate", async () => {
  const app = await show("filters", {});
  await app.waitFor("text:todo 1");
  await app.waitFor("text:todo 2");
  await app.press("id:run-filters");
  expect(app.texts()).toContain("exact 0");
  expect(app.texts()).toContain("partial 2");
  expect(app.texts()).toContain("active 2");
  expect(app.texts()).toContain("inactive 1");
  expect(app.texts()).toContain("stale 1");
  expect(app.texts()).toContain("fresh 2");
  expect(app.texts()).toContain("predicate 1");
});

test("useQueries without combine reads each query by its index, and maps over them", async () => {
  const app = await show("lists", {});
  await app.waitFor("text:list 2 1 2");
  expect(app.shows("text:mapped 1,2")).toBe(true);
});

test("queryOptions, infiniteQueryOptions and mutationOptions return their options unchanged", async () => {
  const app = await show("options", {});
  await app.waitFor("text:options 7");
  await app.waitFor("text:infinite options 10");
  await app.press("id:mutate-options");
  await app.waitFor("text:mutation options 5");
});

test("throwOnError hands the error to the nearest ErrorBoundary", async () => {
  const app = await show("boundary", { network: { [`GET ${API}/thrower`]: { status: 500 } } });
  await app.waitFor("text:boundary status 500");
});

test("every field of the query, infinite and mutation results is present", async () => {
  const app = await show("fields", {});
  await app.waitFor("text:query fields ok");
  expect(app.shows("text:infinite fields ok")).toBe(true);
  expect(app.shows("text:mutation fields ok")).toBe(true);
});

test("a query takes an explicit client, without a provider around it", async () => {
  const app = await show("explicit", {});
  await app.waitFor("text:explicit ok");
});
