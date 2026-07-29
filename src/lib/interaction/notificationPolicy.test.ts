import { describe, expect, it, vi } from 'vitest';
import {
  configureNotifications,
  createNotificationPolicy,
  handleNotificationEvent,
  type NotificationAdapter,
  type PermissionState,
} from './notificationPolicy';

const adapter = (
  permission: PermissionState = 'granted',
): NotificationAdapter => ({
  capability: vi.fn(async () => ({ status: 'available' as const })),
  permission: vi.fn(async () => permission),
  requestPermission: vi.fn(async () => permission),
  send: vi.fn(async () => undefined),
});

describe('native notification policy', () => {
  it('defaults off and opt-out never requests permission', async () => {
    const native = adapter('prompt');
    const state = await configureNotifications(
      createNotificationPolicy(),
      false,
      native,
    );
    expect(state.optedIn).toBe(false);
    expect(native.permission).not.toHaveBeenCalled();
    expect(native.requestPermission).not.toHaveBeenCalled();
  });

  it.each(['granted', 'denied'] as const)(
    'requests prompt permission only during explicit enable and records %s',
    async (result) => {
      const native = adapter('prompt');
      vi.mocked(native.requestPermission).mockResolvedValue(result);
      const state = await configureNotifications(
        createNotificationPolicy(),
        true,
        native,
      );
      expect(native.requestPermission).toHaveBeenCalledOnce();
      expect(state).toMatchObject({ optedIn: true, permission: result });
      expect(state.diagnostic.status).toBe(
        result === 'granted' ? 'available' : 'permission_denied',
      );
    },
  );

  it('does not prompt when permission is already granted or denied', async () => {
    for (const permission of ['granted', 'denied'] as const) {
      const native = adapter(permission);
      await configureNotifications(createNotificationPolicy(), true, native);
      expect(native.requestPermission).not.toHaveBeenCalled();
    }
  });

  it('delivers only primary critical, exhausted, and auth entry', async () => {
    const native = adapter();
    let state = await configureNotifications(
      createNotificationPolicy(),
      true,
      native,
    );
    const events = [
      {
        kind: 'severity_raised',
        provider: 'claude',
        window: 'session',
        severity: 'warn',
      },
      {
        kind: 'severity_raised',
        provider: 'codex',
        window: 'session',
        severity: 'critical',
      },
      {
        kind: 'severity_raised',
        provider: 'claude',
        window: 'session',
        severity: 'critical',
      },
      {
        kind: 'severity_raised',
        provider: 'claude',
        window: 'weekly',
        severity: 'exhausted',
      },
      { kind: 'auth_required', provider: 'claude' },
    ] as const;
    for (const event of events)
      state = await handleNotificationEvent(state, event, 'claude', native);
    expect(native.send).toHaveBeenCalledTimes(3);
  });

  it('deduplicates and reset rearms without primary-switch replay', async () => {
    const native = adapter();
    let state = await configureNotifications(
      createNotificationPolicy(),
      true,
      native,
    );
    const critical = {
      kind: 'severity_raised',
      provider: 'claude',
      window: 'session',
      severity: 'critical',
    } as const;
    state = await handleNotificationEvent(state, critical, 'claude', native);
    state = await handleNotificationEvent(state, critical, 'claude', native);
    state = await handleNotificationEvent(
      state,
      { kind: 'window_reset', provider: 'claude', window: 'session' },
      'claude',
      native,
    );
    state = await handleNotificationEvent(state, critical, 'claude', native);
    const codexAuth = { kind: 'auth_required', provider: 'codex' } as const;
    state = await handleNotificationEvent(state, codexAuth, 'claude', native);
    await handleNotificationEvent(state, codexAuth, 'codex', native);
    expect(native.send).toHaveBeenCalledTimes(2);
  });

  it('reports unavailable capability without requesting permission', async () => {
    const native = adapter('prompt');
    vi.mocked(native.capability).mockResolvedValue({
      status: 'unavailable',
      reason: 'unsupported',
    });
    const state = await configureNotifications(
      createNotificationPolicy(),
      true,
      native,
    );
    expect(state.diagnostic).toEqual({
      status: 'unavailable',
      reason: 'unsupported',
    });
    expect(native.permission).not.toHaveBeenCalled();
    expect(native.requestPermission).not.toHaveBeenCalled();
  });

  it('keeps secondary notifications off by default and labels provider text', async () => {
    const native = adapter();
    let state = await configureNotifications(
      createNotificationPolicy(),
      true,
      native,
    );
    state = await handleNotificationEvent(
      state,
      {
        kind: 'severity_raised',
        provider: 'codex',
        window: 'session',
        severity: 'critical',
      },
      'claude',
      native,
    );
    expect(native.send).not.toHaveBeenCalled();
    expect(state.dedupe).toContain('codex:session:critical');
  });

  it('delivers simultaneous primary and opted-in secondary events with labels', async () => {
    const native = adapter();
    let state = await configureNotifications(
      createNotificationPolicy(),
      true,
      native,
    );
    for (const provider of ['claude', 'codex'] as const)
      state = await handleNotificationEvent(
        state,
        {
          kind: 'severity_raised',
          provider,
          window: 'weekly',
          severity: 'exhausted',
        },
        'claude',
        native,
        true,
      );
    expect(native.send).toHaveBeenNthCalledWith(1, {
      title: 'CacheBite',
      body: 'Claude: Weekly usage is exhausted',
    });
    expect(native.send).toHaveBeenNthCalledWith(2, {
      title: 'CacheBite',
      body: 'Codex: Weekly usage is exhausted',
    });
  });

  it('does not replay an event after primary switch', async () => {
    const native = adapter();
    let state = await configureNotifications(
      createNotificationPolicy(),
      true,
      native,
    );
    const event = {
      kind: 'auth_required',
      provider: 'codex',
    } as const;
    state = await handleNotificationEvent(state, event, 'claude', native);
    await handleNotificationEvent(state, event, 'codex', native);
    expect(native.send).not.toHaveBeenCalled();
  });

  it('reset rearms only its provider namespace', async () => {
    const native = adapter();
    let state = await configureNotifications(
      createNotificationPolicy(),
      true,
      native,
    );
    const eventFor = (provider: 'claude' | 'codex') =>
      ({
        kind: 'severity_raised',
        provider,
        window: 'session',
        severity: 'critical',
      }) as const;
    for (const provider of ['claude', 'codex'] as const)
      state = await handleNotificationEvent(
        state,
        eventFor(provider),
        'claude',
        native,
        true,
      );
    state = await handleNotificationEvent(
      state,
      { kind: 'window_reset', provider: 'codex', window: 'session' },
      'claude',
      native,
      true,
    );
    state = await handleNotificationEvent(
      state,
      eventFor('claude'),
      'claude',
      native,
      true,
    );
    await handleNotificationEvent(
      state,
      eventFor('codex'),
      'claude',
      native,
      true,
    );
    expect(native.send).toHaveBeenCalledTimes(3);
  });
});
