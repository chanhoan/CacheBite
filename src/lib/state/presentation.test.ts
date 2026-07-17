import { describe, expect, it } from 'vitest';

import type { AppSettings } from '../api/gateway';
import type { ProviderUiSnapshot } from '../contracts/domain';
import { applyProviderUpdate, createProviderState } from './engine';
import { toProviderPresentation, toSettingsStoreState } from './presentation';

const settings: AppSettings = {
  schemaVersion: 3,
  primaryProvider: 'claude',
  selectedPetId: 'cat',
  bubblesEnabled: true,
  startAtLogin: false,
  notificationsEnabled: true,
  secondaryNotificationsEnabled: false,
  logicalPosition: { x: 12, y: 34 },
};

const snapshot: ProviderUiSnapshot = {
  provider: 'codex',
  planType: 'Pro',
  session: { usedPercent: 71, windowMinutes: 300, resetsAt: null },
  weekly: { usedPercent: 39, windowMinutes: 10_080, resetsAt: null },
  capturedAt: '2026-07-17T12:00:00Z',
  source: 'cli_rpc',
  isCached: false,
  revision: 1,
  failureClass: null,
  unavailableReason: null,
};

describe('renderer presentation projections', () => {
  it('selects only the settings-store fields from persisted settings', () => {
    expect(toSettingsStoreState(settings)).toEqual({
      primaryProvider: 'claude',
      bubblesEnabled: true,
      startAtLogin: false,
      notificationsEnabled: true,
      secondaryNotificationsEnabled: false,
    });
  });

  it('derives panel and overlay shared fields from one provider state', () => {
    const state = applyProviderUpdate(
      createProviderState('codex'),
      snapshot,
      Date.parse(snapshot.capturedAt),
    ).state;

    expect(
      toProviderPresentation(state, Date.parse(snapshot.capturedAt)),
    ).toMatchObject({
      provider: 'codex',
      system: 'active',
      stale: false,
      planType: 'Pro',
      session: { usedPercent: 71, severity: 'warn', resetsAt: null },
      weekly: { usedPercent: 39, severity: 'ok', resetsAt: null },
      capturedAt: snapshot.capturedAt,
      source: 'cli_rpc',
      isCached: false,
    });
  });
});
