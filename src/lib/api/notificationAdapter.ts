import type {
  NativeNotification,
  NotificationAdapter,
  NotificationDiagnostic,
  PermissionState,
} from '../interaction/notificationPolicy';

/** Narrow facade matching the official Tauri notification plugin API. */
export interface OfficialNotificationApi {
  isPermissionGranted(): Promise<boolean>;
  requestPermission(): Promise<'granted' | 'denied' | 'default'>;
  sendNotification(notification: NativeNotification): void;
}

export class TauriNotificationAdapter implements NotificationAdapter {
  constructor(
    private readonly api: OfficialNotificationApi | null,
    private readonly supported = true,
  ) {}

  async capability(): Promise<NotificationDiagnostic> {
    return this.supported && this.api !== null
      ? { status: 'available' }
      : { status: 'unavailable', reason: 'native notifications unsupported' };
  }

  async permission(): Promise<PermissionState> {
    if (this.api === null) return 'denied';
    return (await this.api.isPermissionGranted()) ? 'granted' : 'prompt';
  }

  async requestPermission(): Promise<PermissionState> {
    if (this.api === null) return 'denied';
    const result = await this.api.requestPermission();
    return result === 'granted' ? 'granted' : 'denied';
  }

  async send(notification: NativeNotification): Promise<void> {
    if (this.api === null) throw new Error('native notifications unavailable');
    this.api.sendNotification(notification);
  }
}

export const nativeNotificationAdapter = new TauriNotificationAdapter({
  isPermissionGranted,
  requestPermission,
  sendNotification,
});
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
