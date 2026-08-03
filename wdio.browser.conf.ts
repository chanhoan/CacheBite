import type { Options } from '@wdio/types';
import { createServer, type ViteDevServer } from 'vite';

const baseUrl = 'http://127.0.0.1:1420';
let viteServer: ViteDevServer | undefined;

async function closeViteServer(): Promise<void> {
  const server = viteServer;
  viteServer = undefined;
  await server?.close();
}

async function startAndWarmViteServer(): Promise<void> {
  viteServer = await createServer({
    server: { host: '127.0.0.1', port: 1420, strictPort: true },
    logLevel: 'warn',
  });
  try {
    await viteServer.listen();
    const response = await fetch(`${baseUrl}/?window=panel&fixture=e2e`);
    if (!response.ok) {
      throw new Error(`Vite warmup failed with HTTP ${response.status}`);
    }
    await response.text();
  } catch (error) {
    await closeViteServer();
    throw error;
  }
}

export const config: Options.Testrunner = {
  runner: 'local',
  specs: [
    './tests/e2e/renderer.spec.ts',
    './tests/e2e/renderer-server.spec.ts',
  ],
  maxInstances: 1,
  framework: 'mocha',
  mochaOpts: {
    timeout: 60_000,
  },
  reporters: ['spec'],
  baseUrl,
  onPrepare: startAndWarmViteServer,
  onComplete: closeViteServer,
  capabilities: [
    {
      browserName: 'chrome',
      'goog:chromeOptions': {
        args: ['--headless=new', '--no-sandbox', '--disable-dev-shm-usage'],
      },
    },
  ],
};
