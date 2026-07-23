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

  it('keeps the overlay transparent, undecorated, and shadowless', () => {
    const config = JSON.parse(
      readFileSync(resolve('src-tauri/tauri.conf.json'), 'utf8'),
    );
    const windows = config.app.windows as Array<{
      label: string;
      transparent?: boolean;
      decorations?: boolean;
      shadow?: boolean;
    }>;

    const overlay = windows.find((window) => window.label === 'overlay');
    const panel = windows.find((window) => window.label === 'panel');

    expect(overlay).toMatchObject({
      transparent: true,
      decorations: false,
      shadow: false,
    });
    expect(panel?.shadow).not.toBe(false);
  });
});
