use std::cell::{Cell, RefCell};

use serde::de::DeserializeOwned;
use tauri::WebviewWindow;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use tokio::sync::oneshot;

use crate::models::{RectEdge, ShareFileOptions, SharePosition, ShareTextOptions, SharedContent};

use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, ProtocolObject},
    AnyThread, DefinedClass,
};
use objc2_app_kit::{
    NSSharingService, NSSharingServicePicker, NSSharingServicePickerDelegate, NSView,
};
use objc2_core_foundation::{CGPoint, CGSize};
use objc2_foundation::{
    NSArray, NSBundle, NSData, NSFileManager, NSObjectProtocol, NSRect, NSRectEdge, NSString,
    NSUserDefaults, NSURL,
};

impl From<RectEdge> for NSRectEdge {
    fn from(edge: RectEdge) -> Self {
        match edge {
            RectEdge::Top => Self::NSMaxYEdge,
            RectEdge::Bottom => Self::NSMinYEdge,
            RectEdge::Left => Self::NSMinXEdge,
            RectEdge::Right => Self::NSMaxXEdge,
        }
    }
}

fn position_to_rect(position: Option<&SharePosition>) -> (f64, f64, NSRectEdge) {
    position.map_or((0.0, 0.0, NSRectEdge::NSMinYEdge), |pos| {
        let edge = pos
            .preferred_edge
            .map_or(NSRectEdge::NSMinYEdge, Into::into);
        (pos.x, pos.y, edge)
    })
}

struct PickerDelegateIvars {
    sender: Cell<Option<oneshot::Sender<crate::Result<()>>>>,
    picker: RefCell<Option<Retained<NSSharingServicePicker>>>,
    retainer: RefCell<Option<Retained<PickerDelegate>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "SharekitPickerDelegate"]
    #[ivars = PickerDelegateIvars]
    struct PickerDelegate;

    unsafe impl NSObjectProtocol for PickerDelegate {}

    unsafe impl NSSharingServicePickerDelegate for PickerDelegate {
        #[unsafe(method(sharingServicePicker:didChooseSharingService:))]
        fn picker_did_choose_service(
            &self,
            _picker: &NSSharingServicePicker,
            service: Option<&NSSharingService>,
        ) {
            if let Some(sender) = self.ivars().sender.take() {
                let result = if service.is_some() {
                    Ok(())
                } else {
                    Err(crate::Error::ShareCancelled)
                };
                let _ = sender.send(result);
            }
            self.ivars().picker.borrow_mut().take();
            // Releases the last strong ref; `self` stays valid for the rest of
            // this method call per ObjC runtime semantics, then deallocs.
            self.ivars().retainer.borrow_mut().take();
        }
    }
);

impl PickerDelegate {
    fn new(sender: oneshot::Sender<crate::Result<()>>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(PickerDelegateIvars {
            sender: Cell::new(Some(sender)),
            picker: RefCell::new(None),
            retainer: RefCell::new(None),
        });
        unsafe { msg_send![super(this), init] }
    }
}

#[allow(clippy::unnecessary_wraps)] // signature required by `lib.rs` plugin setup contract
pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<ShareKit<R>> {
    Ok(ShareKit(app.clone()))
}

/// Access to the share APIs.
pub struct ShareKit<R: Runtime>(AppHandle<R>);

#[allow(clippy::needless_pass_by_value)] // matches public API signatures of `share_text` / `share_file`
fn present_picker<R: Runtime>(
    window: WebviewWindow<R>,
    items_builder: impl FnOnce() -> Vec<Retained<AnyObject>> + Send + 'static,
    position: Option<SharePosition>,
) -> crate::Result<oneshot::Receiver<crate::Result<()>>> {
    let (tx, rx) = oneshot::channel();
    let (x, y, edge) = position_to_rect(position.as_ref());

    window
        .with_webview(move |webview| {
            let ns_view: &NSView = unsafe { &*(webview.inner() as *const NSView) };

            let items = items_builder();
            let items_array = NSArray::from_retained_slice(&items);

            let rect = NSRect::new(CGPoint::new(x, y), CGSize::new(1.0, 1.0));
            let picker = unsafe {
                NSSharingServicePicker::initWithItems(NSSharingServicePicker::alloc(), &items_array)
            };

            let delegate = PickerDelegate::new(tx);
            picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

            // Self-retain delegate + retain picker so they outlive this closure
            // and stay alive until the user dismisses the popover.
            delegate.ivars().picker.borrow_mut().replace(picker.clone());
            delegate
                .ivars()
                .retainer
                .borrow_mut()
                .replace(delegate.clone());

            picker.showRelativeToRect_ofView_preferredEdge(rect, ns_view, edge);
        })
        .map_err(|_| crate::Error::WindowNotFound)?;

    Ok(rx)
}

impl<R: Runtime> ShareKit<R> {
    pub async fn share_text(
        &self,
        window: WebviewWindow<R>,
        text: String,
        options: ShareTextOptions,
    ) -> crate::Result<()> {
        let rx = present_picker(
            window,
            move || {
                let ns_string = NSString::from_str(&text);
                vec![unsafe { Retained::cast_unchecked(ns_string) }]
            },
            options.position,
        )?;

        rx.await.unwrap_or(Err(crate::Error::ShareCancelled))
    }

    pub async fn share_file(
        &self,
        window: WebviewWindow<R>,
        url: String,
        options: ShareFileOptions,
    ) -> crate::Result<()> {
        let rx = present_picker(
            window,
            move || {
                let ns_url = NSURL::fileURLWithPath(&NSString::from_str(&url));
                vec![unsafe { Retained::cast_unchecked(ns_url) }]
            },
            options.position,
        )?;

        rx.await.unwrap_or(Err(crate::Error::ShareCancelled))
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
    Some(format!("group.{bundle_id}"))
}
