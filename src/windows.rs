use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime, WebviewWindow};
use tokio::sync::oneshot;

use crate::models::{ShareFileOptions, ShareTextOptions};
use crate::Error;

use windows::{
    core::{Interface, HSTRING},
    ApplicationModel::DataTransfer::{DataPackage, DataRequestedEventArgs, DataTransferManager},
    Foundation::TypedEventHandler,
    Storage::{IStorageItem, StorageFile},
    Win32::{
        Foundation::HWND,
        System::WinRT::{RoGetActivationFactory, RoInitialize, RO_INIT_SINGLETHREADED},
        UI::Shell::IDataTransferManagerInterop,
    },
};
use windows_collections::IIterable;

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Self::WindowsApi(err.to_string())
    }
}

#[allow(clippy::unnecessary_wraps)] // signature required by `lib.rs` plugin setup contract
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

type CompleteTx = Arc<Mutex<Option<oneshot::Sender<bool>>>>;
type SetupTx = Arc<Mutex<Option<oneshot::Sender<crate::Result<()>>>>>;

fn wire_complete_handlers(
    data: &DataPackage,
    complete_tx: &CompleteTx,
) -> windows::core::Result<()> {
    let tx_completed = complete_tx.clone();
    data.ShareCompleted(&TypedEventHandler::new(move |_, _| {
        if let Some(tx) = tx_completed
            .lock()
            .expect("complete_tx mutex poisoned")
            .take()
        {
            let _ = tx.send(true);
        }
        Ok(())
    }))?;

    let tx_canceled = complete_tx.clone();
    data.ShareCanceled(&TypedEventHandler::new(move |_, _| {
        if let Some(tx) = tx_canceled
            .lock()
            .expect("complete_tx mutex poisoned")
            .take()
        {
            let _ = tx.send(false);
        }
        Ok(())
    }))?;

    Ok(())
}

async fn present_share_ui<F>(
    dtm: &DataTransferManager,
    interop: &IDataTransferManagerInterop,
    hwnd: HWND,
    populate: F,
) -> crate::Result<()>
where
    F: Fn(&DataPackage) -> windows::core::Result<()> + Send + Sync + 'static,
{
    let (setup_tx, setup_rx) = oneshot::channel::<crate::Result<()>>();
    let (complete_tx, complete_rx) = oneshot::channel::<bool>();
    let setup_tx: SetupTx = Arc::new(Mutex::new(Some(setup_tx)));
    let complete_tx: CompleteTx = Arc::new(Mutex::new(Some(complete_tx)));

    let setup_tx_handler = setup_tx.clone();
    let complete_tx_handler = complete_tx.clone();
    let handler: TypedEventHandler<DataTransferManager, DataRequestedEventArgs> =
        TypedEventHandler::new(
            move |_, args: windows::core::Ref<'_, DataRequestedEventArgs>| {
                let result: windows::core::Result<()> = (|| {
                    if let Some(args) = args.as_ref() {
                        let data = args.Request()?.Data()?;
                        populate(&data)?;
                        wire_complete_handlers(&data, &complete_tx_handler)?;
                    }
                    Ok(())
                })();
                if let Some(tx) = setup_tx_handler
                    .lock()
                    .expect("setup_tx mutex poisoned")
                    .take()
                {
                    let _ = tx.send(result.map_err(|e| Error::WindowsApi(e.to_string())));
                }
                Ok(())
            },
        );

    let token = dtm.DataRequested(&handler)?;
    unsafe {
        interop.ShowShareUIForWindow(hwnd)?;
    }

    let setup_result = match setup_rx.await {
        Ok(r) => r,
        Err(e) => Err(Error::WindowsApi(e.to_string())),
    };
    if let Err(e) = setup_result {
        let _ = dtm.RemoveDataRequested(token);
        return Err(e);
    }

    let completed = match complete_rx.await {
        Ok(b) => b,
        Err(e) => {
            let _ = dtm.RemoveDataRequested(token);
            return Err(Error::WindowsApi(e.to_string()));
        }
    };
    let _ = dtm.RemoveDataRequested(token);

    if completed {
        Ok(())
    } else {
        Err(Error::ShareCancelled)
    }
}

impl<R: Runtime> ShareKit<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        // Initialize Windows Runtime if needed
        unsafe {
            let _ = RoInitialize(RO_INIT_SINGLETHREADED);
        }

        Self { app }
    }

    /// Opens the native share UI to share text content.
    pub async fn share_text(
        &self,
        window: WebviewWindow<R>,
        text: String,
        _options: ShareTextOptions,
    ) -> crate::Result<()> {
        let hwnd = window
            .hwnd()
            .map_err(|e| Error::WindowsApi(e.to_string()))?;

        let class = HSTRING::from("Windows.ApplicationModel.DataTransfer.DataTransferManager");
        let interop: IDataTransferManagerInterop = unsafe { RoGetActivationFactory(&class)? };
        let dtm: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };

        let content = HSTRING::from(text);
        let app_name = HSTRING::from(self.app.package_info().name.clone());

        present_share_ui(&dtm, &interop, hwnd, move |data| {
            let props = data.Properties()?;
            props.SetTitle(&app_name)?;
            props.SetDescription(&content)?;
            data.SetText(&content)?;
            Ok(())
        })
        .await
    }

    /// Opens the native share UI to share a file.
    pub async fn share_file(
        &self,
        window: WebviewWindow<R>,
        url: String,
        options: ShareFileOptions,
    ) -> crate::Result<()> {
        let hwnd = window
            .hwnd()
            .map_err(|e| Error::WindowsApi(e.to_string()))?;

        let class = HSTRING::from("Windows.ApplicationModel.DataTransfer.DataTransferManager");
        let interop: IDataTransferManagerInterop = unsafe { RoGetActivationFactory(&class)? };
        let dtm: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };

        let path = HSTRING::from(url);
        let app_name = self.app.package_info().name.clone();
        let title = HSTRING::from(options.title.as_deref().unwrap_or(&app_name));

        present_share_ui(&dtm, &interop, hwnd, move |data| {
            // Title and Description are required for Windows share to work properly
            let props = data.Properties()?;
            props.SetTitle(&title)?;
            props.SetDescription(&title)?;

            let file = StorageFile::GetFileFromPathAsync(&path)?.get()?;
            let storage_item = file.cast::<IStorageItem>()?;
            let storage_items: IIterable<IStorageItem> = vec![Some(storage_item)].into();
            data.SetStorageItemsReadOnly(&storage_items)?;
            Ok(())
        })
        .await
    }
}
