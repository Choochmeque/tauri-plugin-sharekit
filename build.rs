const COMMANDS: &[&str] = &[
    "register_listener",
    "remove_listener",
    "share_text",
    "share_file",
    "get_pending_shared_content",
    "clear_pending_shared_content",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();
}
