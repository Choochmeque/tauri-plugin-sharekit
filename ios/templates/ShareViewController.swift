import UIKit
import Social
import MobileCoreServices
import UniformTypeIdentifiers

class ShareViewController: UIViewController {

    // MARK: - Configuration (will be replaced by setup script)
    private let appGroupIdentifier = "{{APP_GROUP_IDENTIFIER}}"
    private let appURLScheme = "{{APP_URL_SCHEME}}"

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .clear
    }

    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        handleSharedContent()
    }

    private func handleSharedContent() {
        // Check App Groups configuration early
        if FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupIdentifier) == nil {
            showError("App Groups not configured.\n\nPlease enable 'App Groups' capability in Xcode for both the main app and ShareExtension targets, and configure '\(appGroupIdentifier)' in Apple Developer Portal.")
            return
        }

        guard let extensionItems = extensionContext?.inputItems as? [NSExtensionItem] else {
            completeRequest()
            return
        }

        // Use a serial queue to safely collect results
        let resultQueue = DispatchQueue(label: "sharekit.results")
        var sharedContent: [String: Any] = [:]
        var files: [[String: Any]] = []
        var textContent: String? = nil

        let group = DispatchGroup()

        for extensionItem in extensionItems {
            guard let attachments = extensionItem.attachments else { continue }

            for attachment in attachments {
                group.enter()

                if attachment.hasItemConformingToTypeIdentifier(UTType.image.identifier) {
                    // Check image FIRST (before URL) because images can also be URLs
                    attachment.loadItem(forTypeIdentifier: UTType.image.identifier, options: nil) { [weak self] item, error in
                        defer { group.leave() }
                        if let url = item as? URL {
                            if let fileInfo = self?.copyFileToAppGroup(url: url) {
                                resultQueue.sync { files.append(fileInfo) }
                            }
                        } else if let image = item as? UIImage {
                            if let fileInfo = self?.saveImageToAppGroup(image: image) {
                                resultQueue.sync { files.append(fileInfo) }
                            }
                        }
                    }
                } else if attachment.hasItemConformingToTypeIdentifier(UTType.url.identifier) {
                    attachment.loadItem(forTypeIdentifier: UTType.url.identifier, options: nil) { [weak self] item, error in
                        defer { group.leave() }
                        guard let url = item as? URL else { return }

                        if url.isFileURL {
                            if let fileInfo = self?.copyFileToAppGroup(url: url) {
                                resultQueue.sync { files.append(fileInfo) }
                            }
                        } else {
                            resultQueue.sync { textContent = url.absoluteString }
                        }
                    }
                } else if attachment.hasItemConformingToTypeIdentifier(UTType.text.identifier) {
                    attachment.loadItem(forTypeIdentifier: UTType.text.identifier, options: nil) { item, error in
                        defer { group.leave() }
                        if let text = item as? String {
                            resultQueue.sync { textContent = text }
                        }
                    }
                } else if attachment.hasItemConformingToTypeIdentifier(UTType.data.identifier) {
                    attachment.loadItem(forTypeIdentifier: UTType.data.identifier, options: nil) { [weak self] item, error in
                        defer { group.leave() }
                        if let url = item as? URL {
                            if let fileInfo = self?.copyFileToAppGroup(url: url) {
                                resultQueue.sync { files.append(fileInfo) }
                            }
                        }
                    }
                } else {
                    group.leave()
                }
            }
        }

        group.notify(queue: .main) { [weak self] in
            guard let self = self else { return }

            if !files.isEmpty {
                sharedContent["type"] = "files"
                sharedContent["files"] = files
            } else if let text = textContent {
                sharedContent["type"] = "text"
                sharedContent["text"] = text
            }

            if !sharedContent.isEmpty {
                _ = self.saveToAppGroup(content: sharedContent)
                self.openMainAppAndComplete()
            } else {
                self.completeRequest()
            }
        }
    }

    private func showError(_ message: String) {
        let alert = UIAlertController(
            title: "ShareKit Error",
            message: message,
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(title: "OK", style: .default) { [weak self] _ in
            self?.completeRequest()
        })
        present(alert, animated: true)
    }

    private func copyFileToAppGroup(url: URL) -> [String: Any]? {
        guard let containerURL = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupIdentifier) else {
            return nil
        }

        let sharedFilesDir = containerURL.appendingPathComponent("shared_files", isDirectory: true)
        try? FileManager.default.createDirectory(at: sharedFilesDir, withIntermediateDirectories: true)

        let fileName = url.lastPathComponent
        let destinationURL = sharedFilesDir.appendingPathComponent(UUID().uuidString + "_" + fileName)

        do {
            if url.startAccessingSecurityScopedResource() {
                defer { url.stopAccessingSecurityScopedResource() }
                try FileManager.default.copyItem(at: url, to: destinationURL)
            } else {
                try FileManager.default.copyItem(at: url, to: destinationURL)
            }

            var fileInfo: [String: Any] = [
                "path": destinationURL.path,
                "name": fileName
            ]

            if let mimeType = getMimeType(for: url) {
                fileInfo["mimeType"] = mimeType
            }

            if let attributes = try? FileManager.default.attributesOfItem(atPath: destinationURL.path),
               let size = attributes[.size] as? Int64 {
                fileInfo["size"] = size
            }

            return fileInfo
        } catch {
            print("ShareKit: Failed to copy file: \(error)")
            return nil
        }
    }

    private func saveImageToAppGroup(image: UIImage) -> [String: Any]? {
        guard let containerURL = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupIdentifier) else {
            return nil
        }

        let sharedFilesDir = containerURL.appendingPathComponent("shared_files", isDirectory: true)
        try? FileManager.default.createDirectory(at: sharedFilesDir, withIntermediateDirectories: true)

        let fileName = UUID().uuidString + ".png"
        let destinationURL = sharedFilesDir.appendingPathComponent(fileName)

        guard let data = image.pngData() else { return nil }

        do {
            try data.write(to: destinationURL)

            return [
                "path": destinationURL.path,
                "name": fileName,
                "mimeType": "image/png",
                "size": data.count
            ]
        } catch {
            print("ShareKit: Failed to save image: \(error)")
            return nil
        }
    }

    private func getMimeType(for url: URL) -> String? {
        if let uti = UTType(filenameExtension: url.pathExtension) {
            return uti.preferredMIMEType
        }
        return nil
    }

    private func saveToAppGroup(content: [String: Any]) -> Bool {
        guard let userDefaults = UserDefaults(suiteName: appGroupIdentifier) else {
            showError("App Groups not configured.\n\nPlease enable 'App Groups' capability in Xcode for both the main app and ShareExtension targets, and configure '\(appGroupIdentifier)' in Apple Developer Portal.")
            return false
        }

        do {
            let data = try JSONSerialization.data(withJSONObject: content)
            userDefaults.set(data, forKey: "pendingSharedContent")
            userDefaults.synchronize()
            return true
        } catch {
            showError("Failed to save shared content: \(error.localizedDescription)")
            return false
        }
    }

    private func openMainAppAndComplete() {
        guard let url = URL(string: "\(appURLScheme)://sharekit-content") else {
            completeRequest()
            return
        }

        var responder: UIResponder? = self
        while responder != nil {
            if let application = responder as? UIApplication {
                if #available(iOS 18.0, *) {
                    application.open(url, options: [:], completionHandler: nil)
                } else {
                    _ = application.perform(NSSelectorFromString("openURL:"), with: url)
                }
                break
            }
            responder = responder?.next
        }

        completeRequest()
    }

    private func completeRequest() {
        extensionContext?.completeRequest(returningItems: [], completionHandler: nil)
    }
}
