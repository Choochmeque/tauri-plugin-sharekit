use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime, WebviewWindow};

use crate::models::{ShareFileOptions, ShareTextOptions};

#[allow(clippy::unnecessary_wraps)] // signature required by `lib.rs` plugin setup contract
pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<ShareKit<R>> {
    Ok(ShareKit(app.clone()))
}

/// Access to the share APIs.
pub struct ShareKit<R: Runtime>(AppHandle<R>);

// `async` keyword on share_text/share_file is required so `commands.rs` can `.await`
// the call uniformly across platforms; on this stub no await is needed.
impl<R: Runtime> ShareKit<R> {
    #[allow(clippy::unused_async)]
    pub async fn share_text(
        &self,
        _window: WebviewWindow<R>,
        _text: String,
        _options: ShareTextOptions,
    ) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    #[allow(clippy::unused_async)]
    pub async fn share_file(
        &self,
        _window: WebviewWindow<R>,
        _url: String,
        _options: ShareFileOptions,
    ) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn get_pending_shared_content(&self) -> crate::Result<Option<SharedContent>> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn clear_pending_shared_content(&self) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }
}
