import type { Options } from '@wdio/types';
import { resolve } from 'node:path';

const collectorMode = process.env.CACHEBITE_EXPECTED_COLLECTOR_MODE;
if (collectorMode !== 'fixture' && collectorMode !== 'production') {
  throw new Error(
    'Set CACHEBITE_EXPECTED_COLLECTOR_MODE to fixture or production',
  );
}
const application =
  process.env.CACHEBITE_APPLICATION ??
  resolve(
    process.cwd(),
    'src-tauri',
    'target',
    'debug',
    `cachebite${process.platform === 'win32' ? '.exe' : ''}`,
  );
const driverProvider = process.platform === 'darwin' ? 'embedded' : 'official';

export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./tests/e2e/native.spec.ts'],
  maxInstances: 1,
  framework: 'mocha',
  reporters: ['spec'],
  services: [
    [
      'tauri',
      {
        application,
        driverProvider,
        autoInstallTauriDriver: true,
      },
    ],
  ],
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': { application },
    },
  ],
  waitforTimeout: 15_000,
  mochaOpts: {
    timeout: 60_000,
  },
};
