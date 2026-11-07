import { expect, open, test } from "nsh/test";

test("the smoke screen renders with no options", async () => {
  const app = await open("../App.tsx");
  await app.press("id:open-smoke");
  await app.waitFor("text:smoke yes");
  expect(app.texts()).toContain("smoke yes");
});
