import { expect, test } from "nsh/test";
import type { App } from "nsh/test";
import { API, show } from "./support";

/** The number a "label N" text shows now. */
const runsOf = (app: App, label: string): number => {
  const line = app.texts().find((text) => text.startsWith(label));
  return Number(line?.slice(label.length));
};

test("useMutation: a binding on data does not wake when only status changes", async () => {
  const app = await show("fine-mutation", {
    network: { [`POST ${API}/fine-m`]: { body: { n: 1 }, delay: 1000 } },
  });
  await app.waitFor("text:data runs 1");
  await app.waitFor("text:status runs 1");
  await app.press("id:go");
  expect(app.shows("text:status pending")).toBe(true);
  expect(app.shows("text:status runs 2")).toBe(true);
  expect(app.shows("text:data runs 1")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:data runs 2");
  expect(app.shows("text:data 1")).toBe(true);
});

test("useInfiniteQuery: a binding on pages does not wake for a refetch, and does wake for a new page", async () => {
  const app = await show("fine-infinite", {
    network: {
      [`GET ${API}/fine-pages/1`]: { body: { number: 1, next: 2 } },
      [`GET ${API}/fine-pages/2`]: { body: { number: 2, next: null } },
    },
  });
  await app.waitFor("text:pages 1");
  await app.waitFor("text:more true");
  // Measured after the first page arrives, which is itself a change of data and of hasNextPage.
  const data = runsOf(app, "data runs ");
  const next = runsOf(app, "next runs ");

  await app.press("id:refetch");
  expect(runsOf(app, "data runs ")).toBe(data);
  expect(runsOf(app, "next runs ")).toBe(next);

  await app.press("id:next");
  await app.waitFor("text:pages 1,2");
  expect(runsOf(app, "data runs ")).toBe(data + 1);
  expect(runsOf(app, "next runs ")).toBe(next + 1);
});

test("useQueries: a binding on queries[0].data does not wake when another query changes, and the list can grow", async () => {
  let b = 0;
  const app = await show("fine-queries", {
    network: { [`GET ${API}/fine-b`]: () => ({ body: { v: `b${++b}` } }) },
  });
  await app.waitFor("text:a a");
  await app.waitFor("text:b b1");
  const aRuns = runsOf(app, "a runs ");

  await app.press("id:refetch-b");
  await app.waitFor("text:b b2");
  expect(runsOf(app, "a runs ")).toBe(aRuns);

  await app.press("id:grow");
  await app.waitFor("text:length 3");
  await app.waitFor("text:c c");
  expect(app.shows("text:a a")).toBe(true);
});

test("useQueryDiagnostics reports queries and mutations, and wakes only when they change", async () => {
  const app = await show("diagnostics", {
    network: {
      [`GET ${API}/d-a`]: { body: { n: 1 } },
      [`POST ${API}/d-m`]: { body: {}, delay: 1000 },
    },
  });
  await app.waitFor("text:query d-a success idle observers 1 stale false");
  expect(app.shows("text:mutations none")).toBe(true);
  const runs = () => app.texts().find((line) => line.startsWith("diagnostic runs "));
  const before = runs();

  await app.press("id:same");
  expect(runs()).toBe(before);

  await app.press("id:mutate");
  await app.waitFor("text:mutations pending");
  expect(runs()).not.toBe(before);
  await app.clock!.runFor(1000);
  await app.waitFor("text:mutations success");
});
