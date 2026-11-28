import { invokeNative } from '@3pino/app-kit-web/core/nativeInvoke';
import { isAndroid as detectAndroid } from '@3pino/app-kit-web/core/platform';
import type { AndroidSettingKind, InstalledApp, NativeStatus } from './types';

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
  openNotificationSettings: () => call<void>('open_notification_settings'),
  /** Shizuku で「設定の書き込みの権限」を付ける。ok / not_running / too_old / denied / failed */
  grantWithShizuku: async () => (await call<{ result: string }>('grant_with_shizuku')).result,
  requestDeviceAdmin: () => call<void>('request_device_admin'),
  requestBatteryExemption: () => call<void>('request_battery_exemption'),
  requestNotifications: () => call<void>('request_notifications'),
  openAppDetails: () => call<void>('open_app_details'),
  /** この Android の設定の項目へ直接飛ぶ（アプリ情報は経由しない）。開けたら true */
  openAndroidSetting: async (kind: AndroidSettingKind) => (await call<{ opened: boolean }>('open_android_setting', { kind })).opened,
  /** 「アプリ情報」を開いて案内された（「Android の設定」を開く）か。読むと消える */
  takeSettingsRequest: async () => (await call<{ requested: boolean }>('take_settings_request')).requested,
  installedApps: async () => (await call<{ apps: InstalledApp[] }>('installed_apps')).apps,
  /** アプリのアイコン（PNG の base64）。取れなければ空文字 */
  appIcon: async (pkg: string) => (await call<{ png: string }>('app_icon', { pkg })).png,
};
