import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('desktop content security policy', () => {
  it('allows the Tauri Windows IPC transport', () => {
    const config = JSON.parse(
      readFileSync(resolve('src-tauri/tauri.conf.json'), 'utf8'),
    );

    expect(config.app.security.csp).toContain(
      "connect-src 'self' http://ipc.localhost",
    );
  });
});
