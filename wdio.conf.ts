import type { Options } from '@wdio/types';

const collectorMode = process.env.CACHEBITE_EXPECTED_COLLECTOR_MODE;
if (collectorMode !== 'fixture' && collectorMode !== 'production') {
  throw new Error(
    'Set CACHEBITE_EXPECTED_COLLECTOR_MODE to fixture or production',
  );
}
const application = `./src-tauri/target/debug/cachebite${process.platform === 'win32' ? '.exe' : ''}`;

export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./tests/e2e/native.spec.ts'],
  maxInstances: 1,
  framework: 'mocha',
  reporters: ['spec'],
  services: ['tauri'],
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': { application },
    },
  ],
  waitforTimeout: 15_000,
};
