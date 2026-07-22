import { defineConfig } from '@playwright/test';

/**
 * End-to-end tests run against the production build (`pnpm build` first).
 * The environment provides pre-installed browsers at PLAYWRIGHT_BROWSERS_PATH;
 * the pinned chromium revision there may differ from the one this Playwright
 * version bundles, so we point launchOptions at the installed binary directly.
 */
export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  fullyParallel: false,
  use: { baseURL: 'http://localhost:4173' },
  webServer: {
    command: 'pnpm preview --port 4173 --strictPort',
    url: 'http://localhost:4173',
    reuseExistingServer: !process.env.CI,
  },
  projects: [
    {
      name: 'chromium',
      use: {
        browserName: 'chromium',
        launchOptions: { executablePath: '/opt/pw-browsers/chromium' },
      },
    },
  ],
});
