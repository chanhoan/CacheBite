import { describe, expect, it } from 'vitest';
import type {
  UpdateFailureReason,
  UpdateStateWire,
  UpdateStatusWire,
} from '../api/gateway';
import { updateViewModel } from './updatePresentation';

const state = (status: UpdateStatusWire): UpdateStateWire => ({
  currentVersion: '0.1.0-beta.4',
  status,
});

const ALL_REASONS: readonly UpdateFailureReason[] = [
  'offline',
  'rate_limited',
  'metadata_invalid',
  'artifact_missing',
  'download_failed',
  'verification_failed',
  'install_failed',
];

describe('updateViewModel', () => {
  it('hides the banner when the app is up to date', () => {
    const model = updateViewModel(state({ status: 'up_to_date' }), null);

    expect(model.visible).toBe(false);
    expect(model.settingsLine).toBe('Up to date');
  });

  it('reports a distinct settings line before the first check', () => {
    expect(updateViewModel(state({ status: 'idle' }), null).settingsLine).toBe(
      'Not checked yet',
    );
    expect(
      updateViewModel(state({ status: 'checking' }), null).settingsLine,
    ).toBe('Checking…');
  });

  it('marks a check in flight as busy so Settings can disable its button', () => {
    expect(updateViewModel(state({ status: 'checking' }), null).busy).toBe(
      true,
    );
    expect(updateViewModel(state({ status: 'up_to_date' }), null).busy).toBe(
      false,
    );
  });

  it('offers an available update with Install and Later', () => {
    const model = updateViewModel(
      state({
        status: 'available',
        version: '0.1.0-beta.5',
        notes: 'Fixes a crash.',
      }),
      null,
    );

    expect(model).toMatchObject({
      visible: true,
      headline: 'Update available — 0.1.0-beta.5',
      detail: 'Fixes a crash.',
      primaryLabel: 'Install and restart',
      primaryEnabled: true,
      dismissible: true,
      settingsLine: 'Update available — 0.1.0-beta.5',
    });
  });

  it('hides an available update the user dismissed', () => {
    const model = updateViewModel(
      state({ status: 'available', version: '0.1.0-beta.5', notes: null }),
      '0.1.0-beta.5',
    );

    expect(model.visible).toBe(false);
    // Settings still tells the truth — the dismissal only hides the banner.
    expect(model.settingsLine).toBe('Update available — 0.1.0-beta.5');
  });

  it('shows a newer update after an older one was dismissed', () => {
    const model = updateViewModel(
      state({ status: 'available', version: '0.1.0-beta.6', notes: null }),
      '0.1.0-beta.5',
    );

    expect(model.visible).toBe(true);
  });

  it('never hides an in-flight install, whatever was dismissed', () => {
    const downloading = updateViewModel(
      state({ status: 'downloading', received: 50, total: 100 }),
      '0.1.0-beta.5',
    );
    const installing = updateViewModel(
      state({ status: 'installing', version: '0.1.0-beta.5' }),
      '0.1.0-beta.5',
    );

    expect(downloading.visible).toBe(true);
    expect(installing.visible).toBe(true);
  });

  it('never lets a dismissed version hide a failure', () => {
    const model = updateViewModel(
      state({ status: 'failed', reason: 'offline' }),
      '0.1.0-beta.5',
    );

    expect(model.visible).toBe(true);
    expect(model.dismissible).toBe(true);
    expect(model.primaryLabel).toBe('Try again');
  });

  it('hides a failure the user dismissed, but only through the failure flag', () => {
    const dismissed = updateViewModel(
      state({ status: 'failed', reason: 'offline' }),
      null,
      true,
    );

    expect(dismissed.visible).toBe(false);
    // Settings still tells the truth after a dismissal.
    expect(dismissed.settingsLine).toBe('Update failed');
  });

  it('retries a failure with a check, never with an install', () => {
    // The native service refuses to install from anything but `available`, so
    // wiring `Try again` to installUpdate would make the recovery a no-op.
    const failed = updateViewModel(
      state({ status: 'failed', reason: 'offline' }),
      null,
    );

    expect(failed.primaryLabel).toBe('Try again');
    expect(failed.primaryAction).toBe('check');
  });

  it('installs from every state that carries a real offer or in-flight work', () => {
    const installing: UpdateStatusWire[] = [
      { status: 'available', version: '1.0.0', notes: null },
      { status: 'downloading', received: 1, total: 2 },
      { status: 'installing', version: '1.0.0' },
    ];

    for (const status of installing) {
      expect(updateViewModel(state(status), null).primaryAction).toBe(
        'install',
      );
    }
  });

  it('offers no action at all in the states with no banner', () => {
    const quiet: UpdateStatusWire[] = [
      { status: 'idle' },
      { status: 'checking' },
      { status: 'up_to_date' },
    ];

    for (const status of quiet) {
      const model = updateViewModel(state(status), null);
      expect(model.primaryAction).toBeNull();
      expect(model.primaryLabel).toBeNull();
    }
  });

  it('keeps a failure dismissal from suppressing an offer or an install', () => {
    // The two dismissals are independent; the failure flag must not leak.
    expect(
      updateViewModel(
        state({ status: 'available', version: '1.0.0', notes: null }),
        null,
        true,
      ).visible,
    ).toBe(true);
    expect(
      updateViewModel(
        state({ status: 'downloading', received: 1, total: 2 }),
        null,
        true,
      ).visible,
    ).toBe(true);
  });

  it('reports download progress as a percentage', () => {
    const model = updateViewModel(
      state({ status: 'downloading', received: 64, total: 100 }),
      null,
    );

    expect(model.detail).toBe('64%');
    expect(model.primaryEnabled).toBe(false);
  });

  it('reports indeterminate progress when the size is unknown', () => {
    const unknown = updateViewModel(
      state({ status: 'downloading', received: 1024, total: null }),
      null,
    );
    const zero = updateViewModel(
      state({ status: 'downloading', received: 0, total: 0 }),
      null,
    );

    expect(unknown.detail).toBe('Downloading…');
    expect(zero.detail).toBe('Downloading…');
  });

  it('clamps a progress ratio that overshoots its reported total', () => {
    const model = updateViewModel(
      state({ status: 'downloading', received: 300, total: 100 }),
      null,
    );

    expect(model.detail).toBe('100%');
  });

  it('tells the user the app will restart while installing', () => {
    const model = updateViewModel(
      state({ status: 'installing', version: '0.1.0-beta.5' }),
      null,
    );

    expect(model.headline).toBe('Installing 0.1.0-beta.5…');
    expect(model.detail).toBe('CacheBite will restart.');
    expect(model.primaryEnabled).toBe(false);
    expect(model.dismissible).toBe(false);
  });

  it('renders a distinct sentence for every failure reason and leaks no transport detail', () => {
    const sentences = ALL_REASONS.map(
      (reason) =>
        updateViewModel(state({ status: 'failed', reason }), null).detail,
    );

    expect(new Set(sentences).size).toBe(ALL_REASONS.length);
    for (const sentence of sentences) {
      expect(sentence).toBeTruthy();
      expect(sentence).not.toMatch(/http|\/\/|\\|github\.com/i);
    }
  });

  it('always reports a settings line', () => {
    const statuses: UpdateStatusWire[] = [
      { status: 'idle' },
      { status: 'checking' },
      { status: 'up_to_date' },
      { status: 'available', version: '1.0.0', notes: null },
      { status: 'downloading', received: 1, total: 2 },
      { status: 'installing', version: '1.0.0' },
      { status: 'failed', reason: 'offline' },
    ];

    for (const status of statuses) {
      expect(updateViewModel(state(status), null).settingsLine).not.toBe('');
    }
  });
});
