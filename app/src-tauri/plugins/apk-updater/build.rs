// コマンドの実体は Kotlin（android/）にある。ここでは権限の生成だけをする
const COMMANDS: &[&str] = &["can_install", "open_install_settings", "latest_tag", "download_and_install"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
