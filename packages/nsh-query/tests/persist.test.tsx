import { expect, open, test } from "nsh/test";
import { API, show } from "./support";

const KEY = "nsh-query-client";
const saved = { [`GET ${API}/saved`]: { body: { n: 7 } } };

/** Runs the default persisted screen once and returns what it stored. */
async function saveOnce(): Promise<Record<string, string>> {
  const first = await show("persist", { network: saved });
  await first.waitFor("text:saved 7");
  await first.waitUntil(() => first.storage()[KEY] !== undefined);
  return first.storage();
}

test("buster: a stored client with another buster is discarded, not restored", async () => {
  const stored = await saveOnce();
  const second = await show("persist-buster", { storage: stored, network: { [`GET ${API}/saved`]: { body: { n: 8 } } } });
  await second.waitFor("text:saved 8");
  expect(second.requests().length).toBe(1);
});

test("maxAge: an expired stored client is discarded, not restored", async () => {
  const stored = await saveOnce();
  const second = await show("persist-maxage", {
    storage: stored,
    clock: { time: Date.now() + 5000 },
    network: { [`GET ${API}/saved`]: { body: { n: 8 } } },
  });
  await second.waitFor("text:saved 8");
  expect(second.requests().length).toBe(1);
});

test("dehydrateOptions: a query the filter excludes is not stored", async () => {
  const first = await show("persist-filter", {
    network: { ...saved, [`GET ${API}/secret`]: { body: { n: 5 } } },
  });
  await first.waitFor("text:saved 7");
  await first.waitFor("text:secret 5");
  await first.waitUntil(() => first.storage()[KEY] !== undefined);
  const stored = first.storage()[KEY]!;
  expect(stored).toContain('"saved"');
  expect(stored).not.toContain('"secret"');
});

test("hydrateOptions: deserializeData transforms restored data", async () => {
  const stored = await saveOnce();
  const second = await show("persist-hydrate", { storage: stored, network: { [`GET ${API}/saved`]: { body: { n: 99 } } } });
  await second.waitFor("text:saved 700");
  expect(second.requests().length).toBe(0);
});

test("createStoragePersister key: the client is saved under the given key only", async () => {
  const first = await show("persist-key", { network: saved });
  await first.waitFor("text:saved 7");
  await first.waitUntil(() => first.storage()["custom-key"] !== undefined);
  expect(Object.keys(first.storage())).not.toContain(KEY);

  const second = await show("persist-key", { storage: first.storage(), network: { [`GET ${API}/saved`]: { body: { n: 99 } } } });
  await second.waitFor("text:saved 7");
  expect(second.requests().length).toBe(0);
});

test("createStoragePersister throttleTime: a second save waits for the interval", async () => {
  const app = await show("persist-throttle", {});
  await app.press("id:one");
  await app.waitUntil(() => (app.storage()[KEY] ?? "").includes('"one"'));
  await app.press("id:two");
  expect((app.storage()[KEY] ?? "").includes('"two"')).toBe(false);
  await app.clock!.runFor(5000);
  await app.waitUntil(() => (app.storage()[KEY] ?? "").includes('"two"'));
});

test("createStoragePersister serialize and deserialize: a custom format round-trips", async () => {
  const first = await show("persist-codec", { network: saved });
  await first.waitFor("text:saved 7");
  await first.waitUntil(() => first.storage()[KEY] !== undefined);
  expect(first.storage()[KEY]!.startsWith("v2:")).toBe(true);

  const second = await show("persist-codec", { storage: first.storage(), network: { [`GET ${API}/saved`]: { body: { n: 99 } } } });
  await second.waitFor("text:saved 7");
  expect(second.requests().length).toBe(0);
});

test("createAsyncStoragePersister retry: a failed save is tried again, as the retry function asks", async () => {
  const app = await show("persist-retry", { network: saved });
  await app.waitFor("text:retry 1");
  await app.waitUntil(() => app.storage()[KEY] !== undefined);
  expect(app.texts()).not.toContain("retry 2");
});

test("persistQueryClient saves on change, and its promise resolves after the restore", async () => {
  const app = await show("persist-fn", { network: saved });
  await app.waitFor("text:persist done");
  await app.waitFor("text:saved 7");
  await app.press("id:bump");
  await app.waitUntil(() => (app.storage()[KEY] ?? "").includes('"n":8'));
});

test("persistQueryClientRestore restores a stored client into a client", async () => {
  const stored = await saveOnce();
  const second = await show("restore-fn", { storage: stored });
  await second.waitFor('text:restored value {"n":7}');
});

test("experimental_createQueryPersister restores a query from storage", async () => {
  const first = await show("query-persister", { network: { [`GET ${API}/qp`]: { body: { n: 7 } } } });
  await first.waitFor("text:qp 7");
  await first.waitUntil(() => Object.keys(first.storage()).some((key) => key.startsWith("tanstack-query-")));
  const second = await show("query-persister", {
    storage: first.storage(),
    network: { [`GET ${API}/qp`]: { body: { n: 99 } } },
  });
  await second.waitFor("text:qp 7");
  expect(second.requests().length).toBe(0);
});
