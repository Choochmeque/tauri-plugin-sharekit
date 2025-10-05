import LocalAuthentication
import SwiftRs
import Tauri
import UIKit
import WebKit
import SwiftUI
import UIKit
import Foundation

struct ShareOptions: Decodable {
  let text: String
}

struct ShareFileOptions: Decodable {
  let url: String
  let title: String?
}

class SharePlugin: Plugin {
  var webview: WKWebView!
  public override func load(webview: WKWebView) {
    self.webview = webview
  }

  @objc func shareText(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(ShareOptions.self)

    DispatchQueue.main.async {
      let activityViewController = UIActivityViewController(activityItems: [args.text], applicationActivities: nil)

      // Display as popover on iPad as required by apple
      activityViewController.popoverPresentationController?.sourceView = self.webview // display as a popover on ipad
      activityViewController.popoverPresentationController?.sourceRect = CGRect(
        x: self.webview.bounds.midX,
        y: self.webview.bounds.midY,
        width: CGFloat(Float(0.0)),
        height: CGFloat(Float(0.0))
      )

      activityViewController.completionWithItemsHandler = { _, completed, _, error in
        if let error = error {
          invoke.reject(error.localizedDescription)
        } else if completed {
          invoke.resolve()
        } else {
          invoke.reject("Share cancelled")
        }
      }

      self.manager.viewController?.present(activityViewController, animated: true, completion: nil)
    }
  }

  @objc func shareFile(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(ShareFileOptions.self)
    
    DispatchQueue.main.async {
      // Convert URL string to URL object
      guard let fileUrl = URL(string: args.url) else {
        invoke.reject("Invalid file URL")
        return
      }

      let fileManager = FileManager.default
      let tempDirectory = fileManager.temporaryDirectory
      let tempPath = tempDirectory.path + "/" + fileUrl.lastPathComponent
      let tempURL = URL(fileURLWithPath: tempPath)
      try? fileManager.copyItem(atPath:fileUrl.path , toPath: tempPath)

      var activityItems: [Any] = [tempURL]
      
      let activityViewController = UIActivityViewController(
        activityItems: activityItems,
        applicationActivities: nil
      )
      
      // Display as popover on iPad as required by Apple
      activityViewController.popoverPresentationController?.sourceView = self.webview
      activityViewController.popoverPresentationController?.sourceRect = CGRect(
        x: self.webview.bounds.midX,
        y: self.webview.bounds.midY,
        width: CGFloat(Float(0.0)),
        height: CGFloat(Float(0.0))
      )

      activityViewController.completionWithItemsHandler = { _, completed, _, error in
        if let error = error {
          invoke.reject(error.localizedDescription)
        } else if completed {
          invoke.resolve()
        } else {
          invoke.reject("Share cancelled")
        }
      }

      self.manager.viewController?.present(activityViewController, animated: true, completion: nil)
    }
  }
}

@_cdecl("init_plugin_share")
func initPlugin() -> Plugin {
  return SharePlugin()
}
