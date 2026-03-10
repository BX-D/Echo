import { expect, test } from "@playwright/test";

async function skipTypewriter(page: import("@playwright/test").Page) {
  await page.keyboard.press("Space");
  await page.waitForTimeout(700);
}

async function clickFirstChoice(page: import("@playwright/test").Page) {
  const panel = page.getByTestId("choice-panel");
  await expect(panel).toBeVisible();
  await page.waitForTimeout(700);
  await panel.locator("button").first().click();
}

async function satisfyMediaPrompts(page: import("@playwright/test").Page) {
  const cameraButton = page.getByRole("button", { name: /enable camera/i });
  if (await cameraButton.isVisible().catch(() => false)) {
    await cameraButton.click();
    await page.waitForTimeout(1000);
  }

  const micButton = page.getByRole("button", { name: /enable mic/i });
  if (await micButton.isVisible().catch(() => false)) {
    await micButton.click();
    await page.waitForTimeout(1000);
  }
}

test("Start -> Session Beats -> Dynamic Surface -> Reveal", async ({
  page,
}) => {
  test.setTimeout(5 * 60 * 1000);

  await page.goto("/");

  await expect(page.getByText(/it learns your fear/i)).toBeVisible();
  await page.getByRole("button", { name: /press enter to begin/i }).click();

  await expect(page.getByTestId("game-screen")).toBeVisible();
  await expect(page.getByTestId("session-chrome")).toBeVisible();
  await satisfyMediaPrompts(page);

  for (let step = 0; step < 6; step += 1) {
    await skipTypewriter(page);
    await satisfyMediaPrompts(page);
    const hasChoicePanel = await page
      .getByTestId("choice-panel")
      .isVisible()
      .catch(() => false);
    if (hasChoicePanel) {
      await clickFirstChoice(page);
    }
  }

  const currentScene = page.getByTestId("current-scene");
  await expect(currentScene).toBeVisible();
  await expect(page.getByTestId("session-chrome")).toBeVisible();

  const image = page.locator('[data-testid="scene-image"] img').first();
  if (await image.isVisible().catch(() => false)) {
    await expect(image).toBeVisible();
  }

  for (let step = 0; step < 16; step += 1) {
    if (await page.getByTestId("fear-reveal").isVisible().catch(() => false)) {
      break;
    }

    await skipTypewriter(page);
    await satisfyMediaPrompts(page);
    const hasChoicePanel = await page
      .getByTestId("choice-panel")
      .isVisible()
      .catch(() => false);
    if (!hasChoicePanel) {
      continue;
    }
    await clickFirstChoice(page);
  }

  await expect(page.getByTestId("fear-reveal")).toBeVisible({
    timeout: 3 * 60 * 1000,
  });
  await expect(page.getByText(/session verdict/i)).toBeVisible();
  await expect(page.getByTestId("fear-summary")).toBeVisible();
  await expect(page.getByTestId("fear-chart")).toBeVisible();
  await expect(page.getByTestId("ending-classification")).toBeVisible({
    timeout: 15_000,
  });
  await expect(page.getByTestId("analysis-closing")).toBeVisible();
});
