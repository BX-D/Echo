import { defineConfig, devices } from "@playwright/test";
import { fileURLToPath } from "node:url";
import path from "path";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  retries: 0,
  timeout: 5 * 60 * 1000,
  expect: {
    timeout: 30 * 1000,
  },
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: "http://127.0.0.1:4173",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
    permissions: ["camera"],
    launchOptions: {
      args: [
        "--use-fake-ui-for-media-stream",
        "--use-fake-device-for-media-stream",
      ],
    },
  },
  webServer: [
    {
      command: "./scripts/start-e2e-backend.sh",
      cwd: dirname,
      url: "http://127.0.0.1:3002/health",
      reuseExistingServer: false,
      timeout: 2 * 60 * 1000,
    },
    {
      command: "./scripts/start-e2e-frontend.sh",
      cwd: dirname,
      url: "http://127.0.0.1:4173",
      reuseExistingServer: false,
      timeout: 2 * 60 * 1000,
    },
  ],
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        channel: "chromium",
      },
    },
  ],
  outputDir: path.join(dirname, "test-results"),
});
