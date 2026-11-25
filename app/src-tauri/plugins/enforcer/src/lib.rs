//! Android の強制。Accessibility Service が前面のアプリを見て制限し、
//! フォアグラウンドサービスがサーバーの WebSocket につないでハートビートと利用時間を送る。
//! コマンドは Rust 側で処理せず、そのまま Kotlin のプラグインに渡る。

use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("enforcer")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            _api.register_android_plugin("dev.offsave.enforcer", "EnforcerPlugin")?;
            Ok(())
        })
        .build()
}
