use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Share<R>> {
    Ok(Share(app.clone()))
}

/// Access to the share APIs.
pub struct Share<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Share<R> {
    pub fn share_text(&self, _text: String, _options: ShareTextOptions) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn share_file(&self, _url: String, _options: ShareFileOptions) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }
}