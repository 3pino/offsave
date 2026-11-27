import { invokeNative } from '@3pino/app-kit-web/core/nativeInvoke';
import { isAndroid as detectAndroid } from '@3pino/app-kit-web/core/platform';
import type { InstalledApp, NativeStatus } from './types';

/** Android の強制サービス（Kotlin の enforcer プラグイン）。Android 以外では使えない。 */
export const isAndroid = detectAndroid();

const call = <T>(cmd: string, args?: Record<string, unknown>) => invokeNative<T>('enforcer', cmd, args);

export const native = {
  status: () => call<NativeStatus>('status'),
  configure: (a: {
    serverUrl: string;
    devUser: string | null;
    deviceId: string;
    token: string;
    userId: string;
    guardSettings: boolean;
  }) => call<NativeStatus>('configure', a),
  clear: () => call<NativeStatus>('clear'),
  openAccessibilitySettings: () => call<void>('open_accessibility_settings'),
  openUsageAccessSettings: () => call<void>('open_usage_access_settings'),
  requestDeviceAdmin: () => call<void>('request_device_admin'),
  requestBatteryExemption: () => call<void>('request_battery_exemption'),
  requestNotifications: () => call<void>('request_notifications'),
  openAppDetails: () => call<void>('open_app_details'),
  installedApps: async () => (await call<{ apps: InstalledApp[] }>('installed_apps')).apps,
  /** アプリのアイコン（PNG の base64）。取れなければ空文字 */
  appIcon: async (pkg: string) => (await call<{ png: string }>('app_icon', { pkg })).png,
};
