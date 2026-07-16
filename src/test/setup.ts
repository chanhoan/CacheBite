import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, beforeEach } from 'vitest';

beforeEach(() => {
  mockIPC((command) => {
    throw new Error(`Unexpected native IPC in renderer test: ${command}`);
  });
});

afterEach(() => {
  clearMocks();
});
