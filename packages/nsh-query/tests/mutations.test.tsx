import { expect, test } from "nsh/test";
import { API, show } from "./support";

test("onMutate context reaches onError and onSettled, for optimistic update and rollback", async () => {
  const app = await show("optimistic", {
    network: {
      [`GET ${API}/liked`]: { body: { liked: false } },
      [`POST ${API}/like`]: { status: 500, delay: 1000 },
    },
  });
  await app.waitFor("text:liked false");
  await app.press("id:like");
  await app.waitFor("text:liked true");
  await app.clock!.runFor(1000);
  await app.waitFor("text:liked false");
  await app.waitFor("text:like error yes");
  expect(app.texts()).toContain("rolled back to false");
  expect(app.texts()).toContain("settled false");
});

test("invalidateQueries from onSuccess, mutateAsync result and rejection, and reset", async () => {
  let stored = 5;
  const app = await show("save", {
    network: {
      [`GET ${API}/server`]: () => ({ body: { n: stored } }),
      [`POST ${API}/server`]: (request) => {
        const body = JSON.parse(request.body!) as { n: number };
        if (body.n < 0) return { status: 400 };
        stored = body.n;
        return { body: { n: body.n }, delay: 1000 };
      },
    },
  });
  await app.waitFor("text:server 5");

  await app.press("id:save");
  expect(app.shows("text:status pending")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:server 9");
  expect(app.shows("text:status success")).toBe(true);

  await app.press("id:async");
  await app.clock!.runFor(1000);
  await app.waitFor("text:async 7");
  await app.waitFor("text:server 7");

  await app.press("id:reject");
  await app.waitFor("text:rejected status 400");
  expect(app.shows("text:status error")).toBe(true);

  await app.press("id:reset");
  expect(app.shows("text:status idle")).toBe(true);
});

test("mutation retry, and scope running calls in series", async () => {
  let attempts = 0;
  const app = await show("retrying", {
    network: {
      [`POST ${API}/flaky-save`]: () => (++attempts < 3 ? { status: 500 } : { body: {} }),
      [`POST ${API}/serial/a`]: { body: {}, delay: 1000 },
      [`POST ${API}/serial/b`]: { body: {}, delay: 1000 },
    },
  });
  await app.press("id:flaky");
  expect(app.shows("text:flaky pending 1")).toBe(true);
  await app.clock!.runFor(1000);
  expect(app.shows("text:flaky pending 2")).toBe(true);
  await app.clock!.runFor(1000);
  await app.waitFor("text:flaky success 0");
  expect(attempts).toBe(3);

  const serial = () => app.requests().filter((request) => request.url.includes("/serial/")).map((request) => request.url);
  await app.press("id:serial");
  expect(serial()).toEqual([`${API}/serial/a`]);
  await app.clock!.runFor(1000);
  expect(serial()).toEqual([`${API}/serial/a`, `${API}/serial/b`]);
});

test("a mutation in flight finishes after its component goes", async () => {
  const app = await show("leave", { network: { [`POST ${API}/leave`]: { body: {}, delay: 500 } } });
  await app.waitFor("text:board initial");
  await app.press("id:leave-save");
  await app.press("id:hide");
  expect(app.shows("text:leave save")).toBe(false);
  await app.clock!.runFor(600);
  await app.waitFor("text:board saved");
});
