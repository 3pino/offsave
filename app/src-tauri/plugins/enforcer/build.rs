// コマンドの実体は Kotlin（android/）にある。ここでは権限の生成だけをする
const COMMANDS: &[&str] = &[
    "configure",
    "clear",
    "status",
    "open_accessibility_settings",
    "request_device_admin",
    "request_battery_exemption",
    "request_notifications",
    "open_app_details",
    "installed_apps",
    "app_icon",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
