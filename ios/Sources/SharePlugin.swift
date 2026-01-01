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

// Shared content types for receiving shares
struct SharedFile: Codable {
  let path: String
  let name: String
  let mimeType: String?
  let size: Int64?
}

struct SharedContent: Codable {
  let type: String
  let text: String?
  let files: [SharedFile]?
}

class SharePlugin: Plugin {
  var webview: WKWebView!
  private var appGroupId: String?

  public override func load(webview: WKWebView) {
    self.webview = webview

    // Register for app becoming active to check for shared content
    NotificationCenter.default.addObserver(
      self,
      selector: #selector(applicationDidBecomeActive),
      name: UIApplication.didBecomeActiveNotification,
      object: nil
    )
  }

  @objc private func applicationDidBecomeActive() {
    checkAndTriggerSharedContent()
  }

  private func getAppGroupId() -> String? {
    if let groupId = appGroupId {
      return groupId
    }

    if let bundleId = Bundle.main.bundleIdentifier {
      appGroupId = "group.\(bundleId)"
      return appGroupId
    }

    return nil
  }

  private func checkAndTriggerSharedContent() {
    guard let groupId = getAppGroupId(),
          let userDefaults = UserDefaults(suiteName: groupId),
          let data = userDefaults.data(forKey: "pendingSharedContent") else {
      return
    }

    do {
      let content = try JSONDecoder().decode(SharedContent.self, from: data)
      try trigger("sharedContent", data: content)
    } catch {
      print("ShareKit: Failed to parse shared content: \(error)")
    }
  }

  @objc func getPendingSharedContent(_ invoke: Invoke) {
    guard let groupId = getAppGroupId(),
          let userDefaults = UserDefaults(suiteName: groupId),
          let data = userDefaults.data(forKey: "pendingSharedContent") else {
      invoke.resolve()
      return
    }

    do {
      let content = try JSONDecoder().decode(SharedContent.self, from: data)
      invoke.resolve(content)
    } catch {
      invoke.reject("Failed to parse shared content: \(error.localizedDescription)")
    }
  }

  @objc func clearPendingSharedContent(_ invoke: Invoke) {
    guard let groupId = getAppGroupId(),
          let userDefaults = UserDefaults(suiteName: groupId) else {
      invoke.resolve()
      return
    }

    userDefaults.removeObject(forKey: "pendingSharedContent")
    userDefaults.synchronize()

    // Clean up shared files
    if let containerURL = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: groupId) {
      let sharedFilesDir = containerURL.appendingPathComponent("shared_files", isDirectory: true)
      try? FileManager.default.removeItem(at: sharedFilesDir)
    }

    invoke.resolve()
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

      let activityItems: [Any] = [tempURL]
      
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
