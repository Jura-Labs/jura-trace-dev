// SPDX-License-Identifier: AGPL-3.0-or-later

import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 30_000,
  retries: 0,
  fullyParallel: true,
  reporter: 'list',

  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
  },

  projects: [
    {
      name: 'chromium-desktop',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 1280, height: 720 },
      },
    },
    {
      name: 'chromium-mobile',
      use: {
        ...devices['Pixel 5'],
        viewport: { width: 375, height: 812 },
      },
    },
  ],

  webServer: {
    // `npx vite dev`, not `npm run dev`.
    //
    // npm runs the `predev` script automatically, and predev is
    //   cargo run --manifest-path=../src-tauri/Cargo.toml --bin gen-detectors
    // so starting the e2e server compiled the Rust tree. On a runner with no
    // cargo cache that takes minutes and the 60-second timeout below expired
    // long before Vite was ever reached. The failure message says only
    // "Timed out waiting 60000ms from config.webServer", which gives no hint
    // that a Rust build is the reason.
    //
    // Vite alone is enough here: `src/lib/generated/expectedDetectors.ts` is
    // committed, so nothing needs generating before the suite runs.
    //
    // The regeneration that predev does is still worth having during real
    // development, because it catches drift between the Rust detector lineup
    // and the frontend's expectations. Bypassing it is right for e2e and
    // wrong as a way to check that drift; a CI comparison of the committed
    // file against gen-detectors output belongs in the Rust job, which
    // already has a warm cargo cache. Tracked in BL-TEST-003.
    command: 'npx vite dev',
    url: 'http://localhost:1420',
    reuseExistingServer: true,
    // Vite starts in about a second locally. The headroom is for a cold
    // runner installing nothing but still paging in dependencies.
    timeout: 120_000,
  },
});
