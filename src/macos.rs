use objc2::AnyThread;
use serde::de::DeserializeOwned;
use tauri::Manager;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

use objc2::{rc::Retained, runtime::AnyObject};
use objc2_app_kit::{NSSharingServicePicker, NSWindow};
use objc2_core_foundation::{CGPoint, CGSize};
use objc2_foundation::{NSArray, NSRect, NSRectEdge, NSString, NSURL};

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<ShareKit<R>> {
    Ok(ShareKit(app.clone()))
}

/// Access to the share APIs.
pub struct ShareKit<R: Runtime>(AppHandle<R>);

impl<R: Runtime> ShareKit<R> {
    pub fn share_text(&self, text: String, _options: ShareTextOptions) -> crate::Result<()> {
        use std::sync::{Arc, Mutex};

        let app_handle = self.0.clone();
        let error_holder = Arc::new(Mutex::new(None));
        let error_holder_clone = error_holder.clone();

        self.0
            .run_on_main_thread(move || {
                let result = (|| -> crate::Result<()> {
                    let window = app_handle
                        .get_webview_window("main")
                        .ok_or(crate::Error::WindowNotFound)?;
                    let ns_window = window
                        .ns_window()
                        .map_err(|_| crate::Error::WindowNotFound)?
                        as *mut NSWindow;
                    let content_view = unsafe {
                        ns_window
                            .as_ref()
                            .ok_or(crate::Error::WindowNotFound)?
                            .contentView()
                            .ok_or(crate::Error::WindowNotFound)?
                    };

                    let mut items: Vec<Retained<AnyObject>> = Vec::new();
                    let ns_string = NSString::from_str(&text);
                    items.push(unsafe { Retained::cast_unchecked(ns_string) });

                    // NSArray from Vec
                    let items_array = NSArray::from_retained_slice(&items);

                    let rect = NSRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
                    let picker = unsafe {
                        NSSharingServicePicker::initWithItems(
                            NSSharingServicePicker::alloc(),
                            &items_array,
                        )
                    };

                    picker.showRelativeToRect_ofView_preferredEdge(
                        rect,
                        &content_view,
                        NSRectEdge::NSMinYEdge,
                    );

                    Ok(())
                })();

                if let Err(e) = result {
                    *error_holder_clone.lock().unwrap() = Some(e);
                }
            })
            .map_err(|_| crate::Error::UnsupportedPlatform)?;

        if let Some(error) = error_holder.lock().unwrap().take() {
            return Err(error);
        }

        Ok(())
    }

    pub fn share_file(&self, url: String, _options: ShareFileOptions) -> crate::Result<()> {
        use std::sync::{Arc, Mutex};

        let app_handle = self.0.clone();
        let error_holder = Arc::new(Mutex::new(None));
        let error_holder_clone = error_holder.clone();

        self.0
            .run_on_main_thread(move || {
                let result = (|| -> crate::Result<()> {
                    let window = app_handle
                        .get_webview_window("main")
                        .ok_or(crate::Error::WindowNotFound)?;
                    let ns_window = window
                        .ns_window()
                        .map_err(|_| crate::Error::WindowNotFound)?
                        as *mut NSWindow;
                    let content_view = unsafe {
                        ns_window
                            .as_ref()
                            .ok_or(crate::Error::WindowNotFound)?
                            .contentView()
                            .ok_or(crate::Error::WindowNotFound)?
                    };

                    let mut items: Vec<Retained<AnyObject>> = Vec::new();
                    let ns_url = NSURL::fileURLWithPath(&NSString::from_str(&url));
                    items.push(unsafe { Retained::cast_unchecked(ns_url) });

                    // NSArray from Vec
                    let items_array = NSArray::from_retained_slice(&items);

                    let rect = NSRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
                    let picker = unsafe {
                        NSSharingServicePicker::initWithItems(
                            NSSharingServicePicker::alloc(),
                            &items_array,
                        )
                    };

                    picker.showRelativeToRect_ofView_preferredEdge(
                        rect,
                        &content_view,
                        NSRectEdge::NSMinYEdge,
                    );

                    Ok(())
                })();

                if let Err(e) = result {
                    *error_holder_clone.lock().unwrap() = Some(e);
                }
            })
            .map_err(|_| crate::Error::UnsupportedPlatform)?;

        if let Some(error) = error_holder.lock().unwrap().take() {
            return Err(error);
        }

        Ok(())
    }

    pub fn get_pending_shared_content(&self) -> crate::Result<Option<SharedContent>> {
        Ok(None)
    }

    pub fn clear_pending_shared_content(&self) -> crate::Result<()> {
        Ok(())
    }
}
