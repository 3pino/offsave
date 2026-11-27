// アプリ自身のアップデートの状態（土台 @3pino/app-kit-web/updater の Updater を、Tauri の仕組みでつないだもの）
import { isTauri } from '@3pino/app-kit-web/core/platform';
import { Updater } from '@3pino/app-kit-web/updater';
import { tauriUpdateBackend } from './updateBackend';

export { RELEASES_URL } from './updateBackend';

export const updater = new Updater(tauriUpdateBackend(), isTauri());
