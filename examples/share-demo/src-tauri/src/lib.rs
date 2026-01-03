#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sharekit::init())
        .setup(|app| {
            // Windows Share Target handling
            #[cfg(target_os = "windows")]
            {
                use tauri_plugin_sharekit::ShareExt;
                if app.share().handle_share_activation()? {
                    app.handle().exit(0);
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
