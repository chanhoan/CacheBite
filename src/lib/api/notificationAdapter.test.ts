import { describe, expect, it, vi } from 'vitest';
import {
  TauriNotificationAdapter,
  type OfficialNotificationApi,
} from './notificationAdapter';

const api = (): OfficialNotificationApi => ({
  isPermissionGranted: vi.fn(async () => false),
  requestPermission: vi.fn(async () => 'default' as const),
  sendNotification: vi.fn(),
});

describe('Tauri notification adapter boundary', () => {
  it('maps official prompt/default and sends allowlisted title/body only', async () => {
    const official = api();
    const adapter = new TauriNotificationAdapter(official);
    expect(await adapter.permission()).toBe('prompt');
    expect(await adapter.requestPermission()).toBe('denied');
    await adapter.send({ title: 'CacheBite', body: 'Sign in required' });
    expect(official.sendNotification).toHaveBeenCalledWith({
      title: 'CacheBite',
      body: 'Sign in required',
    });
  });

  it('reports capability degradation independently', async () => {
    expect(
      await new TauriNotificationAdapter(null, false).capability(),
    ).toEqual({
      status: 'unavailable',
      reason: 'native notifications unsupported',
    });
  });
});
