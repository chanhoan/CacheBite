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
      width?: number;
      transparent?: boolean;
      decorations?: boolean;
      resizable?: boolean;
      shadow?: boolean;
    }>;

    const overlay = windows.find((window) => window.label === 'overlay');
    const panel = windows.find((window) => window.label === 'panel');

    expect(overlay).toMatchObject({
      transparent: true,
      decorations: false,
      shadow: false,
    });
    expect(panel).toMatchObject({
      width: 312,
      transparent: true,
      decorations: false,
      resizable: false,
    });
    expect(panel?.shadow).not.toBe(false);
  });

  it('contains no platform-specific panel style branches', () => {
    const globalStyles = readFileSync(
      resolve('src/lib/styles/global.css'),
      'utf8',
    );

    expect(globalStyles).not.toMatch(/main\.panel\[data-platform=/);
    expect(globalStyles).not.toContain("font-family: 'Segoe UI'");
    expect(globalStyles).not.toContain("font-family: 'Cantarell'");
  });

  it('does not grant renderer windows direct native resize permission', () => {
    const panelCapability = JSON.parse(
      readFileSync(resolve('src-tauri/capabilities/panel.json'), 'utf8'),
    ) as { permissions: string[] };
    const overlayCapability = JSON.parse(
      readFileSync(resolve('src-tauri/capabilities/overlay.json'), 'utf8'),
    ) as { permissions: string[] };

    expect(panelCapability.permissions).not.toContain(
      'core:window:allow-set-size',
    );
    expect(overlayCapability.permissions).not.toContain(
      'core:window:allow-set-size',
    );
  });

  it('pins the updater to signed manifests on the CacheBite release host', () => {
    const config = JSON.parse(
      readFileSync(resolve('src-tauri/tauri.conf.json'), 'utf8'),
    );

    expect(config.plugins.updater.pubkey).toMatch(/\S/);
    for (const endpoint of config.plugins.updater.endpoints as string[]) {
      expect(new URL(endpoint).protocol).toBe('https:');
      expect(new URL(endpoint).host).toBe('github.com');
    }
    expect(
      config.plugins.updater.dangerousInsecureTransportProtocol,
    ).toBeUndefined();
    // CI turns this on through the generated release config; committing `true`
    // would break every local build that lacks the signing key.
    expect(config.bundle.createUpdaterArtifacts).toBe(false);
  });

  it('does not grant renderer windows direct updater plugin permission', () => {
    // The renderer reaches updates only through `window::command_allowed`.
    // An `updater:*` permission would hand the webview a bypass around it.
    for (const file of ['overlay.json', 'panel.json']) {
      const capability = JSON.parse(
        readFileSync(resolve(`src-tauri/capabilities/${file}`), 'utf8'),
      ) as { permissions: string[] };

      expect(
        capability.permissions.some((permission) =>
          permission.startsWith('updater:'),
        ),
      ).toBe(false);
    }
  });
});
