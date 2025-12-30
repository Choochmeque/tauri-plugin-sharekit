use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::ShareExt;

#[command]
pub async fn share_text<R: Runtime>(
    app: AppHandle<R>,
    text: String,
    mime_type: Option<String>,
) -> Result<(), String> {
    app.share()
        .share_text(text, ShareTextOptions { mime_type })
        .map_err(|e| e.to_string())
}

#[command]
pub async fn share_file<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    mime_type: Option<String>,
    title: Option<String>,
) -> Result<(), String> {
    app.share()
        .share_file(url, ShareFileOptions { mime_type, title })
        .map_err(|e| e.to_string())
}

#[command]
pub async fn get_pending_shared_content<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<SharedContent>, String> {
    app.share()
        .get_pending_shared_content()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn clear_pending_shared_content<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    app.share()
        .clear_pending_shared_content()
        .map_err(|e| e.to_string())
}
