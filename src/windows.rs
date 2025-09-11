use serde::de::DeserializeOwned;
use tauri::Manager;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;
use crate::Error;

use std::sync::mpsc;

use windows::{
    core::{Interface, HSTRING},
    ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager},
    Foundation::TypedEventHandler,
    Storage::{IStorageItem, StorageFile},
    Win32::{
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

    pub fn share_text(&self, text: String, _options: ShareTextOptions) -> crate::Result<()> {
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

        // Create a channel to signal when DataRequested has been called
        let (tx, rx) = mpsc::channel::<()>();

        // Set up the DataRequested handler
        let content = HSTRING::from(text);
        let handler: TypedEventHandler<DataTransferManager, DataRequestedEventArgs> =
            TypedEventHandler::new(
                move |_, args: windows::core::Ref<'_, DataRequestedEventArgs>| {
                    if let Some(args) = args.as_ref() {
                        let data = args.Request()?.Data()?;
                        data.SetText(&content)?;
                    }

                    // Signal that the callback has been executed
                    let _ = tx.send(());

                    Ok(())
                },
            );
        let token = dtm.DataRequested(&handler)?;

        // Show the Share UI for this window
        unsafe {
            interop.ShowShareUIForWindow(hwnd)?;
        }

        let _ = rx.recv();

        let _ = dtm.RemoveDataRequested(token);

        Ok(())
    }

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

        // Create a channel to signal when DataRequested has been called
        let (tx, rx) = mpsc::channel::<()>();

        // Set up the DataRequested handler
        let path = HSTRING::from(url);
        let handler: TypedEventHandler<DataTransferManager, DataRequestedEventArgs> =
            TypedEventHandler::new(
                move |_, args: windows::core::Ref<'_, DataRequestedEventArgs>| {
                    let args = args.as_ref();
                    if let Some(args) = args {
                        let request = args.Request()?;
                        let data = request.Data()?;

                        if let Some(ref title) = options.title {
                            data.Properties()?.SetTitle(&HSTRING::from(title))?;
                        }

                        // Convert path -> StorageFile
                        let file = StorageFile::GetFileFromPathAsync(&path)?.get()?;

                        let storage_item = file.cast::<IStorageItem>()?;
                        let storage_items: IIterable<IStorageItem> =
                            vec![Some(storage_item)].into();

                        data.SetStorageItemsReadOnly(&storage_items)?;
                    }

                    // Signal that the callback has been executed
                    let _ = tx.send(());

                    Ok(())
                },
            );
        let token = dtm.DataRequested(&handler)?;

        // Show the Share UI for this window
        unsafe {
            interop.ShowShareUIForWindow(hwnd)?;
        }

        let _ = rx.recv();

        let _ = dtm.RemoveDataRequested(token);

        Ok(())
    }
}
