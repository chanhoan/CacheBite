import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  resolve: {
    conditions: ['browser'],
  },
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Cargo writes into src-tauri/target while `tauri dev` compiles; on
      // Windows, watching those transient build artifacts crashes Vite's
      // watcher with EBUSY before the app ever launches.
      ignored: ['**/src-tauri/**'],
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['src/test/setup.ts'],
    include: ['src/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/**/*.{ts,svelte}'],
      exclude: [
        // Vite bootstrap has no application behavior; App.svelte is covered.
        'src/main.ts',
        // Ambient declarations and test harness setup contain no runtime logic.
        'src/vite-env.d.ts',
        'src/test/setup.ts',
        // Renderer fixture is exercised by E2E tests, not production unit coverage.
        'src/lib/api/fixtureGateway.ts',
        // Type-only view-model contracts produce no executable application code.
        'src/lib/components/models.ts',
        'src/lib/contracts/domain.ts',
      ],
      thresholds: {
        branches: 80,
        functions: 80,
        lines: 80,
        statements: 80,
      },
    },
  },
});
