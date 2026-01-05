use serde::de::DeserializeOwned;
use tauri::Manager;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

use objc2::{rc::Retained, runtime::AnyObject, AnyThread};
use objc2_app_kit::{NSSharingServicePicker, NSView};
use objc2_core_foundation::{CGPoint, CGSize};
use objc2_foundation::{
    NSArray, NSBundle, NSData, NSFileManager, NSRect, NSRectEdge, NSString, NSUserDefaults, NSURL,
};

impl From<RectEdge> for NSRectEdge {
    fn from(edge: RectEdge) -> Self {
        match edge {
            RectEdge::Top => NSRectEdge::NSMaxYEdge,
            RectEdge::Bottom => NSRectEdge::NSMinYEdge,
            RectEdge::Left => NSRectEdge::NSMinXEdge,
            RectEdge::Right => NSRectEdge::NSMaxXEdge,
        }
    }
}

fn position_to_rect(position: Option<&SharePosition>) -> (f64, f64, NSRectEdge) {
    position
        .map(|pos| {
            let edge = pos
                .preferred_edge
                .map(Into::into)
                .unwrap_or(NSRectEdge::NSMinYEdge);
            (pos.x, pos.y, edge)
        })
        .unwrap_or((0.0, 0.0, NSRectEdge::NSMinYEdge))
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<ShareKit<R>> {
    Ok(ShareKit(app.clone()))
}

/// Access to the share APIs.
pub struct ShareKit<R: Runtime>(AppHandle<R>);

impl<R: Runtime> ShareKit<R> {
    pub fn share_text(&self, text: String, options: ShareTextOptions) -> crate::Result<()> {
        let window = self
            .0
            .get_webview_window("main")
            .ok_or(crate::Error::WindowNotFound)?;

        window
            .with_webview(move |webview| {
                // Get the WKWebView as NSView
                let ns_view: &NSView = unsafe { &*(webview.inner() as *const NSView) };

                let mut items: Vec<Retained<AnyObject>> = Vec::new();
                let ns_string = NSString::from_str(&text);
                items.push(unsafe { Retained::cast_unchecked(ns_string) });

                let items_array = NSArray::from_retained_slice(&items);

                let (x, y, edge) = position_to_rect(options.position.as_ref());

                let rect = NSRect::new(CGPoint::new(x, y), CGSize::new(1.0, 1.0));
                let picker = unsafe {
                    NSSharingServicePicker::initWithItems(
                        NSSharingServicePicker::alloc(),
                        &items_array,
                    )
                };

                picker.showRelativeToRect_ofView_preferredEdge(rect, ns_view, edge);
            })
            .map_err(|_| crate::Error::WindowNotFound)?;

        Ok(())
    }

    pub fn share_file(&self, url: String, options: ShareFileOptions) -> crate::Result<()> {
        let window = self
            .0
            .get_webview_window("main")
            .ok_or(crate::Error::WindowNotFound)?;

        window
            .with_webview(move |webview| {
                // Get the WKWebView as NSView
                let ns_view: &NSView = unsafe { &*(webview.inner() as *const NSView) };

                let mut items: Vec<Retained<AnyObject>> = Vec::new();
                let ns_url = NSURL::fileURLWithPath(&NSString::from_str(&url));
                items.push(unsafe { Retained::cast_unchecked(ns_url) });

                let items_array = NSArray::from_retained_slice(&items);

                let (x, y, edge) = position_to_rect(options.position.as_ref());

                let rect = NSRect::new(CGPoint::new(x, y), CGSize::new(1.0, 1.0));
                let picker = unsafe {
                    NSSharingServicePicker::initWithItems(
                        NSSharingServicePicker::alloc(),
                        &items_array,
                    )
                };

                picker.showRelativeToRect_ofView_preferredEdge(rect, ns_view, edge);
            })
            .map_err(|_| crate::Error::WindowNotFound)?;

        Ok(())
    }

    pub fn get_pending_shared_content(&self) -> crate::Result<Option<SharedContent>> {
        // Get the App Group ID based on bundle identifier
        let group_id = match get_app_group_id() {
            Some(id) => id,
            None => return Ok(None),
        };

        // Get UserDefaults for the App Group
        let ns_group_id = NSString::from_str(&group_id);
        let user_defaults =
            NSUserDefaults::initWithSuiteName(NSUserDefaults::alloc(), Some(&ns_group_id));

        let Some(user_defaults) = user_defaults else {
            return Ok(None);
        };

        // Read the pending shared content data
        let key = NSString::from_str("pendingSharedContent");
        let data: Option<Retained<NSData>> = user_defaults.dataForKey(&key);

        let Some(data) = data else {
            return Ok(None);
        };

        // Convert NSData to bytes and parse JSON
        let bytes = data.to_vec();
        let json_str = std::str::from_utf8(&bytes).map_err(|e| {
            crate::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid UTF-8 in shared content: {e}"),
            ))
        })?;

        let content: SharedContent = serde_json::from_str(json_str).map_err(|e| {
            crate::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse shared content JSON: {e}"),
            ))
        })?;

        Ok(Some(content))
    }

    pub fn clear_pending_shared_content(&self) -> crate::Result<()> {
        // Get the App Group ID based on bundle identifier
        let group_id = match get_app_group_id() {
            Some(id) => id,
            None => return Ok(()),
        };

        // Get UserDefaults for the App Group
        let ns_group_id = NSString::from_str(&group_id);
        let user_defaults =
            NSUserDefaults::initWithSuiteName(NSUserDefaults::alloc(), Some(&ns_group_id));

        let Some(user_defaults) = user_defaults else {
            return Ok(());
        };

        // Remove the pending shared content
        let key = NSString::from_str("pendingSharedContent");
        user_defaults.removeObjectForKey(&key);
        user_defaults.synchronize();

        // Clean up shared files directory
        let file_manager = NSFileManager::defaultManager();
        let container_url = file_manager
            .containerURLForSecurityApplicationGroupIdentifier(&NSString::from_str(&group_id));

        if let Some(container_url) = container_url {
            let shared_files_url = container_url
                .URLByAppendingPathComponent_isDirectory(&NSString::from_str("shared_files"), true);

            if let Some(shared_files_url) = shared_files_url {
                // Try to remove the directory, ignore errors (it might not exist)
                let _ = file_manager.removeItemAtURL_error(&shared_files_url);
            }
        }

        Ok(())
    }
}

/// Get the App Group identifier based on the bundle identifier.
/// Returns `group.{bundle_id}` format.
fn get_app_group_id() -> Option<String> {
    let main_bundle = NSBundle::mainBundle();
    let bundle_id = main_bundle.bundleIdentifier()?;
    Some(format!("group.{}", bundle_id.to_string()))
}
