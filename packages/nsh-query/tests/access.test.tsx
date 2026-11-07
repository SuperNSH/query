import { expect, open, test } from "nsh/test";
import { API, show } from "./support";

test("a capability refusal (ERR_ACCESS_DENIED) reaches query.error, and does not retry forever", async () => {
  const app = await show("denied", { network: { [`GET ${API}/denied`]: { body: {} } } });
  // Default retry is 3, with delays of 1 s, 2 s and 4 s: four attempts in 7 s, then the error.
  await app.clock!.runFor(7000);
  await app.waitFor("text:error ERR_ACCESS_DENIED");
  expect(app.shows("text:attempts 4")).toBe(true);
});

test("persistence: a query restored from Storage after reopening with the same storage option", async () => {
  const first = await show("persist", { network: { [`GET ${API}/saved`]: { body: { n: 7 } } } });
  await first.waitFor("text:saved 7");
  await first.waitUntil(() => first.storage()["nsh-query-client"] !== undefined);
  const stored = first.storage();

  const second = await show("persist", { storage: stored });
  await second.waitFor("text:saved 7");
  expect(second.requests().length).toBe(0);
  expect(second.texts()).toContain("restored");

  const third = await open("../App.tsx", { clock: {}, storage: {} });
  await third.press("id:open-persist");
  expect(third.shows("text:saved 7")).toBe(false);
});

test("PersistQueryClientProvider's onError runs when restoring throws", async () => {
  const app = await show("persist-fail", {});
  await app.waitFor("text:saved none");
  expect(app.texts()).toContain("restore failed");
});
