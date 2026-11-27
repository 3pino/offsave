#[cfg(not(any(target_os = "android", target_os = "ios")))]
use tauri::Manager;

/// 開発者ツールを開く（画面の Ctrl+Shift+I や F12 から呼ぶ。配布版でも使える。Android では何もしない）
#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow) {
    #[cfg(desktop)]
    window.open_devtools();
    #[cfg(not(desktop))]
    let _ = window;
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![open_devtools])
        .plugin(tauri_plugin_opener::init());

    // 2 つ目の起動は、新しく立ち上げず、既存のウィンドウを前に出す
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));

    #[cfg(any(windows, target_os = "linux"))]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    #[cfg(target_os = "android")]
    let builder = builder
        .plugin(tauri_plugin_enforcer::init())
        .plugin(tauri_plugin_apk_updater::init());

    builder
        .run(tauri::generate_context!())
        .expect("offsave の起動に失敗しました");
}
