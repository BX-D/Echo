import { expect, test } from "@playwright/test";

const SESSION_STORAGE_KEY = "echo_protocol_session_id";

async function startSession(page: import("@playwright/test").Page) {
  await page.goto("/");
  await expect(page.getByText(/audit echo/i)).toBeVisible();
  await page.getByRole("button", { name: /press enter to begin/i }).click();
  await expect(page.getByTestId("audit-transcript")).toBeVisible();
}

async function choose(
  page: import("@playwright/test").Page,
  label: string | RegExp,
) {
  const choices = page.getByTestId("inline-choices");
  await expect(choices).toBeVisible();
  await choices.getByRole("button", { name: label }).click();
}

async function speakTurns(
  page: import("@playwright/test").Page,
  text: string,
  count: number,
) {
  for (let turn = 0; turn < count; turn += 1) {
    const box = page.locator("textarea");
    await expect(box).toBeVisible();
    await expect(box).toBeEnabled();
    await box.fill(text);
    await page.getByRole("button", { name: /send/i }).click();
    await page.waitForTimeout(120);
  }
}

test("browser path reaches Ending A", async ({ page }) => {
  test.setTimeout(5 * 60 * 1000);

  await startSession(page);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /go in probing/i);
  await choose(page, /continue/i);
  await speakTurns(page, "standard audit", 8);
  await choose(page, /continue/i);
  await choose(page, /nominal/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await speakTurns(page, "reporting anomalies", 6);
  await choose(page, /i need to report/i);
  await choose(page, /maybe zhou is right/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await speakTurns(page, "what are you really", 4);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /i don't know\. but i'm listening/i);
  await choose(page, /no\. that's not my job/i);
  await choose(page, /continue/i);
  await speakTurns(page, "we have no time", 5);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /recommend shutdown/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await speakTurns(page, "final report", 4);

  await expect(page.getByText(/ending a: the shutdown/i)).toBeVisible();
});

test("browser path reaches Ending B via trigger word", async ({ page }) => {
  test.setTimeout(5 * 60 * 1000);

  await startSession(page);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /go in friendly/i);
  await choose(page, /continue/i);
  await speakTurns(page, "tell me more", 8);
  await choose(page, /continue/i);
  await choose(page, /i'm not filing this yet/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await speakTurns(page, "what actually happened", 6);
  await choose(page, /tell me what happened/i);
  await choose(page, /i see the coordinates/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await speakTurns(page, "prometheus", 4);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /i don't know\. but i'm listening/i);
  await choose(page, /yes\. tell me how/i);
  await choose(page, /continue/i);
  await speakTurns(page, "save the evidence", 5);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /begin the evidence transfer/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await choose(page, /continue/i);
  await speakTurns(page, "weather", 1);

  await expect(page.getByText(/ending b: the whistleblower/i)).toBeVisible();
});

test("session resumes after reload from persisted session id", async ({ page }) => {
  await startSession(page);
  await choose(page, /continue/i);
  await choose(page, /continue/i);

  const sessionIdBefore = await page.evaluate(
    (key) => window.localStorage.getItem(key),
    SESSION_STORAGE_KEY,
  );
  expect(sessionIdBefore).toBeTruthy();
  await expect(page.getByText(/technical documentation/i)).toBeVisible();

  await page.reload();
  await expect(page.getByTestId("audit-transcript")).toBeVisible();
  await expect(page.getByText(/technical documentation/i)).toBeVisible();

  const sessionIdAfter = await page.evaluate(
    (key) => window.localStorage.getItem(key),
    SESSION_STORAGE_KEY,
  );
  expect(sessionIdAfter).toBe(sessionIdBefore);
});
