//! Declares the application's IPC commands so that Tauri generates one
//! `allow-<command>` permission per command. The capability files under
//! `capabilities/` then grant each window only what it needs.

fn main() {
    let commands: &[&str] = &[
        "launch_context",
        "pick",
        "dismiss",
        "open_settings",
        "get_config",
        "save_config",
        "discover_browsers",
        "test_url",
        "default_browser_status",
        "register_default_browser",
        "import_hurl",
        "app_info",
        "check_latest_release",
        "open_release_page",
    ];
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(commands));
    if let Err(error) = tauri_build::try_build(attributes) {
        eprintln!("tauri build failed: {error}");
        std::process::exit(1);
    }
}
