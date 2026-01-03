use serde::de::DeserializeOwned;
use tauri::Manager;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::Error;

use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};

use windows::{
    core::{Interface, HSTRING},
    ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager},
    ApplicationModel::{
        Activation::{ActivationKind, IActivatedEventArgs, ShareTargetActivatedEventArgs},
        AppInstance,
    },
    Foundation::TypedEventHandler,
    Storage::{IStorageItem, StorageFile, StorageFolder},
    Win32::{
        Storage::Packaging::Appx::GetCurrentPackageFullName,
        System::WinRT::{RoGetActivationFactory, RoInitialize, RO_INIT_SINGLETHREADED},
        UI::Shell::IDataTransferManagerInterop,
    },
};
use windows_collections::IIterable;

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error::WindowsApi(err.to_string())
    }
}

// Global storage for pending shared content (cold start scenario)
static PENDING_CONTENT: OnceLock<Mutex<Option<SharedContent>>> = OnceLock::new();

fn get_pending_content_store() -> &'static Mutex<Option<SharedContent>> {
    PENDING_CONTENT.get_or_init(|| Mutex::new(None))
}

/// Returns true if the app is running as an MSIX packaged app.
/// Share Target functionality is only available for packaged apps.
fn is_msix_packaged() -> bool {
    let mut length: u32 = 0;
    // GetCurrentPackageFullName returns ERROR_INSUFFICIENT_BUFFER if packaged
    // and APPMODEL_ERROR_NO_PACKAGE if not packaged
    let result = unsafe { GetCurrentPackageFullName(&mut length, None) };
    // If we got ERROR_INSUFFICIENT_BUFFER (buffer too small), we're packaged
    result.is_err() && length > 0
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<ShareKit<R>> {
    Ok(ShareKit::new(app.clone()))
}

/// Access to the share APIs.
pub struct ShareKit<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> ShareKit<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        // Initialize Windows Runtime if needed
        unsafe {
            let _ = RoInitialize(RO_INIT_SINGLETHREADED);
        }

        Self { app }
    }

    /// Handle Windows Share Target activation.
    ///
    /// Call this in Tauri's setup hook. Returns:
    /// - `Ok(false)`: This is the main instance, continue running
    /// - `Ok(true)`: This is secondary instance, activation forwarded, should exit
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// tauri::Builder::default()
    ///     .plugin(tauri_plugin_sharekit::init())
    ///     .setup(|app| {
    ///         #[cfg(target_os = "windows")]
    ///         if app.share().handle_share_activation()? {
    ///             app.handle().exit(0);
    ///         }
    ///         Ok(())
    ///     })
    ///     .run(tauri::generate_context!())
    /// ```
    pub fn handle_share_activation(&self) -> crate::Result<bool> {
        if !is_msix_packaged() {
            log::debug!("ShareKit: Not an MSIX packaged app, share target unavailable");
            return Ok(false);
        }

        let key = HSTRING::from("ShareKit_Instance");
        let instance = AppInstance::FindOrRegisterForKey(&key)?;

        if instance.IsCurrent()? {
            // We are the main instance
            log::debug!("ShareKit: Main instance, setting up share target handlers");
            self.setup_activated_handler()?;
            self.check_initial_activation()?;
            return Ok(false);
        }

        // We are a secondary instance - redirect activation to main and exit
        log::debug!("ShareKit: Secondary instance, redirecting to main");
        let current = AppInstance::GetCurrent()?;
        if let Ok(Some(args)) = current.GetActivatedEventArgs() {
            let _ = instance.RedirectActivationToAsync(&args)?.get();
        }
        Ok(true)
    }

    /// Set up handler for warm start (app already running, receives new share).
    fn setup_activated_handler(&self) -> crate::Result<()> {
        let current = AppInstance::GetCurrent()?;
        let cache_dir = self.get_cache_dir()?;

        let handler = TypedEventHandler::new(
            move |_sender, args: windows::core::Ref<'_, IActivatedEventArgs>| {
                if let Some(args) = args.as_ref() {
                    if let Ok(kind) = args.Kind() {
                        if kind == ActivationKind::ShareTarget {
                            log::info!("ShareKit: Received share target activation (warm start)");
                            if let Ok(share_args) = args.cast::<ShareTargetActivatedEventArgs>() {
                                match extract_shared_content(&share_args, &cache_dir) {
                                    Ok(content) => {
                                        // Report completed to Windows
                                        if let Ok(op) = share_args.ShareOperation() {
                                            let _ = op.ReportCompleted();
                                        }

                                        // Trigger event for JavaScript listeners
                                        if let Ok(payload) = serde_json::to_string(&content) {
                                            let _ =
                                                crate::listeners::trigger("sharedContent", payload);
                                        }
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "ShareKit: Failed to extract shared content: {}",
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(())
            },
        );

        current.Activated(&handler)?;
        Ok(())
    }

    /// Check if app was launched via share target (cold start).
    fn check_initial_activation(&self) -> crate::Result<()> {
        let current = AppInstance::GetCurrent()?;
        let args = current.GetActivatedEventArgs();

        if let Ok(Some(args)) = args {
            if let Ok(kind) = args.Kind() {
                if kind == ActivationKind::ShareTarget {
                    log::info!("ShareKit: App launched via share target (cold start)");
                    let share_args: ShareTargetActivatedEventArgs = args.cast()?;
                    let cache_dir = self.get_cache_dir()?;
                    let content = extract_shared_content(&share_args, &cache_dir)?;

                    // Store for retrieval via get_pending_shared_content()
                    if let Ok(mut guard) = get_pending_content_store().lock() {
                        *guard = Some(content);
                    }

                    // Report completed to Windows
                    share_args.ShareOperation()?.ReportCompleted()?;
                }
            }
        }

        Ok(())
    }

    /// Get the cache directory for storing shared files.
    fn get_cache_dir(&self) -> crate::Result<std::path::PathBuf> {
        self.app
            .path()
            .app_cache_dir()
            .map_err(|e| Error::WindowsApi(format!("Failed to get cache dir: {}", e)))
    }

    /// Opens the native share UI to share text content.
    pub fn share_text(&self, text: String, _options: ShareTextOptions) -> crate::Result<()> {
        // Get the window handle from Tauri
        let window = self
            .app
            .get_webview_window("main")
            .ok_or(crate::Error::WindowNotFound)?;
        let hwnd = window
            .hwnd()
            .map_err(|e| crate::Error::WindowsApi(e.to_string()))?;

        // Get DataTransferManager bound to our window
        let class = HSTRING::from("Windows.ApplicationModel.DataTransfer.DataTransferManager");
        let interop: IDataTransferManagerInterop = unsafe { RoGetActivationFactory(&class)? };
        let dtm: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };

        // setup_tx: signals when data preparation is done (with success/error)
        // complete_tx: signals when user completes or cancels the share
        let (setup_tx, setup_rx) = mpsc::channel::<Result<(), String>>();
        let (complete_tx, complete_rx) = mpsc::channel::<bool>();

        // Set up the DataRequested handler
        let content = HSTRING::from(text);
        let app_name = HSTRING::from(self.app.package_info().name.clone());
        let handler: TypedEventHandler<DataTransferManager, DataRequestedEventArgs> =
            TypedEventHandler::new(
                move |_, args: windows::core::Ref<'_, DataRequestedEventArgs>| {
                    let result = (|| -> windows::core::Result<()> {
                        if let Some(args) = args.as_ref() {
                            let data = args.Request()?.Data()?;
                            let props = data.Properties()?;
                            props.SetTitle(&app_name)?;
                            props.SetDescription(&content)?;
                            data.SetText(&content)?;

                            // Set up ShareCompleted handler
                            let tx_completed = complete_tx.clone();
                            data.ShareCompleted(&TypedEventHandler::new(move |_, _| {
                                let _ = tx_completed.send(true);
                                Ok(())
                            }))?;

                            // Set up ShareCanceled handler
                            let tx_canceled = complete_tx.clone();
                            data.ShareCanceled(&TypedEventHandler::new(move |_, _| {
                                let _ = tx_canceled.send(false);
                                Ok(())
                            }))?;
                        }
                        Ok(())
                    })();
                    let _ = setup_tx.send(result.map_err(|e| e.to_string()));
                    Ok(())
                },
            );
        let token = dtm.DataRequested(&handler)?;

        // Show the native share UI
        unsafe {
            interop.ShowShareUIForWindow(hwnd)?;
        }

        // Wait for data preparation to complete
        let setup_result = setup_rx
            .recv()
            .map_err(|e| Error::WindowsApi(e.to_string()))
            .and_then(|r| r.map_err(Error::WindowsApi));

        if let Err(e) = setup_result {
            let _ = dtm.RemoveDataRequested(token);
            return Err(e);
        }

        // Wait for user to complete or cancel the share
        let completed = complete_rx
            .recv()
            .map_err(|e| Error::WindowsApi(e.to_string()))?;
        let _ = dtm.RemoveDataRequested(token);

        if completed {
            Ok(())
        } else {
            Err(Error::WindowsApi("Share cancelled".to_string()))
        }
    }

    /// Opens the native share UI to share a file.
    pub fn share_file(&self, url: String, options: ShareFileOptions) -> crate::Result<()> {
        // Get the window handle from Tauri
        let window = self
            .app
            .get_webview_window("main")
            .ok_or(crate::Error::WindowNotFound)?;
        let hwnd = window
            .hwnd()
            .map_err(|e| crate::Error::WindowsApi(e.to_string()))?;

        // Get IDataTransferManagerInterop factory
        let class = HSTRING::from("Windows.ApplicationModel.DataTransfer.DataTransferManager");
        let interop: IDataTransferManagerInterop = unsafe { RoGetActivationFactory(&class)? };

        // Get DataTransferManager bound to our HWND
        let dtm: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };

        // Create channels for signaling
        let (setup_tx, setup_rx) = mpsc::channel::<Result<(), String>>();
        let (complete_tx, complete_rx) = mpsc::channel::<bool>(); // true = completed, false = cancelled

        // Set up the DataRequested handler
        let path = HSTRING::from(url);
        let app_name = self.app.package_info().name.clone();
        let handler: TypedEventHandler<DataTransferManager, DataRequestedEventArgs> =
            TypedEventHandler::new(
                move |_, args: windows::core::Ref<'_, DataRequestedEventArgs>| {
                    let result = (|| -> windows::core::Result<()> {
                        let args = args.as_ref();
                        if let Some(args) = args {
                            let request = args.Request()?;
                            let data = request.Data()?;

                            // Title and Description are required for Windows share to work properly
                            let title = options.title.as_deref().unwrap_or(&app_name);
                            let props = data.Properties()?;
                            props.SetTitle(&HSTRING::from(title))?;
                            props.SetDescription(&HSTRING::from(title))?;

                            // Convert path -> StorageFile
                            let file = StorageFile::GetFileFromPathAsync(&path)?.get()?;

                            let storage_item = file.cast::<IStorageItem>()?;
                            let storage_items: IIterable<IStorageItem> =
                                vec![Some(storage_item)].into();

                            data.SetStorageItemsReadOnly(&storage_items)?;

                            // Set up ShareCompleted handler
                            let tx_completed = complete_tx.clone();
                            data.ShareCompleted(&TypedEventHandler::new(move |_, _| {
                                let _ = tx_completed.send(true);
                                Ok(())
                            }))?;

                            // Set up ShareCanceled handler
                            let tx_canceled = complete_tx.clone();
                            data.ShareCanceled(&TypedEventHandler::new(move |_, _| {
                                let _ = tx_canceled.send(false);
                                Ok(())
                            }))?;
                        }
                        Ok(())
                    })();

                    // Signal setup completion with result
                    let _ = setup_tx.send(result.map_err(|e| e.to_string()));

                    Ok(())
                },
            );
        let token = dtm.DataRequested(&handler)?;

        // Show the Share UI for this window
        unsafe {
            interop.ShowShareUIForWindow(hwnd)?;
        }

        // Wait for setup to complete and check for errors
        let setup_result = setup_rx
            .recv()
            .map_err(|e| Error::WindowsApi(e.to_string()))
            .and_then(|r| r.map_err(Error::WindowsApi));

        if let Err(e) = setup_result {
            let _ = dtm.RemoveDataRequested(token);
            return Err(e);
        }

        // Wait for share to complete or be cancelled
        let completed = complete_rx
            .recv()
            .map_err(|e| Error::WindowsApi(e.to_string()))?;
        let _ = dtm.RemoveDataRequested(token);

        if completed {
            Ok(())
        } else {
            Err(Error::WindowsApi("Share cancelled".to_string()))
        }
    }

    pub fn get_pending_shared_content(&self) -> crate::Result<Option<SharedContent>> {
        if let Ok(guard) = get_pending_content_store().lock() {
            Ok(guard.clone())
        } else {
            Ok(None)
        }
    }

    pub fn clear_pending_shared_content(&self) -> crate::Result<()> {
        // Clear the pending content
        if let Ok(mut guard) = get_pending_content_store().lock() {
            *guard = None;
        }

        // Clean up cached files
        if let Ok(cache_dir) = self.get_cache_dir() {
            let shared_dir = cache_dir.join("shared_files");
            let _ = std::fs::remove_dir_all(&shared_dir);
        }

        Ok(())
    }
}

/// Extract shared content from ShareTargetActivatedEventArgs.
fn extract_shared_content(
    share_args: &ShareTargetActivatedEventArgs,
    cache_dir: &std::path::Path,
) -> crate::Result<SharedContent> {
    let share_op = share_args.ShareOperation()?;
    let data = share_op.Data()?;

    // Check for text content
    let text_format = HSTRING::from("Text");
    if data.Contains(&text_format)? {
        let text_async = data.GetTextAsync()?;
        let text = text_async.get()?;

        return Ok(SharedContent {
            content_type: SharedContentType::Text,
            text: Some(text.to_string()),
            files: None,
        });
    }

    // Check for storage items (files)
    let storage_format = HSTRING::from("StorageItems");
    if data.Contains(&storage_format)? {
        let items_async = data.GetStorageItemsAsync()?;
        let items = items_async.get()?;

        let mut shared_files = Vec::new();
        let shared_dir = cache_dir.join("shared_files");
        std::fs::create_dir_all(&shared_dir)?;

        for i in 0..items.Size()? {
            if let Ok(item) = items.GetAt(i) {
                if let Ok(file) = item.cast::<StorageFile>() {
                    if let Ok(shared_file) = copy_file_to_cache(&file, &shared_dir) {
                        shared_files.push(shared_file);
                    }
                }
            }
        }

        if !shared_files.is_empty() {
            return Ok(SharedContent {
                content_type: SharedContentType::Files,
                text: None,
                files: Some(shared_files),
            });
        }
    }

    Err(Error::WindowsApi("No shareable content found".to_string()))
}

/// Copy a StorageFile to the cache directory.
fn copy_file_to_cache(
    file: &StorageFile,
    cache_dir: &std::path::Path,
) -> crate::Result<SharedFile> {
    let name = file.Name()?.to_string();
    let content_type = file.ContentType().ok().map(|s| s.to_string());

    // Get file size
    let props = file.GetBasicPropertiesAsync()?.get()?;
    let size = props.Size()?;

    // Generate unique filename to avoid collisions
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dest_name = format!("{}_{}", timestamp, name);
    let dest_path = cache_dir.join(&dest_name);

    // Get destination folder as StorageFolder
    let cache_dir_str = cache_dir.to_string_lossy();
    let dest_folder =
        StorageFolder::GetFolderFromPathAsync(&HSTRING::from(cache_dir_str.as_ref()))?.get()?;

    // Copy file to cache
    file.CopyAsync(&dest_folder)?.get()?;

    Ok(SharedFile {
        path: dest_path.to_string_lossy().to_string(),
        name,
        mime_type: content_type,
        size: Some(size),
    })
}
