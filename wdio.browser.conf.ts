import type { Options } from '@wdio/types';

export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./tests/e2e/renderer.spec.ts'],
  maxInstances: 1,
  framework: 'mocha',
  reporters: ['spec'],
  baseUrl: 'http://127.0.0.1:1420',
  capabilities: [
    {
      browserName: 'chrome',
      'goog:chromeOptions': {
        args: ['--headless=new', '--no-sandbox', '--disable-dev-shm-usage'],
      },
    },
  ],
};
