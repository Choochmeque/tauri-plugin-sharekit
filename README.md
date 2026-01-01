[![npm](https://img.shields.io/npm/v/@choochmeque/tauri-plugin-sharekit-api.svg)](https://www.npmjs.com/package/@choochmeque/tauri-plugin-sharekit-api)
[![Crates.io](https://img.shields.io/crates/v/tauri-plugin-sharekit.svg)](https://crates.io/crates/tauri-plugin-sharekit)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/Choochmeque/tauri-plugin-sharekit/blob/main/LICENSE)

# Tauri Plugin ShareKit

Share content to other apps via native sharing interfaces on Android, iOS, macOS and Windows.

## Install

_This plugin requires a Rust version of at least **1.65**_

There are three general methods of installation that we can recommend.

1. Use crates.io and npm (easiest, and requires you to trust that our publishing pipeline worked)
2. Pull sources directly from Github using git tags / revision hashes (most secure)
3. Git submodule install this repo in your tauri project and then use file protocol to ingest the source (most secure, but inconvenient to use)

Install the Core plugin by adding the following to your `Cargo.toml` file:

`src-tauri/Cargo.toml`

```toml
[dependencies]
tauri-plugin-sharekit = "0.2"
# alternatively with Git:
tauri-plugin-sharekit = { git = "https://github.com/Choochmeque/tauri-plugin-sharekit" }
```

You can install the JavaScript Guest bindings using your preferred JavaScript package manager:

<!-- Add the branch for installations using git! -->

```sh
pnpm add @choochmeque/tauri-plugin-sharekit-api
# or
npm add @choochmeque/tauri-plugin-sharekit-api
# or
yarn add @choochmeque/tauri-plugin-sharekit-api

# alternatively with Git:
pnpm add https://github.com/Choochmeque/tauri-plugin-sharekit
# or
npm add https://github.com/Choochmeque/tauri-plugin-sharekit
# or
yarn add https://github.com/Choochmeque/tauri-plugin-sharekit
```

## Usage

First you need to register the core plugin with Tauri:

`src-tauri/src/main.rs`

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sharekit::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Afterwards all the plugin's APIs are available through the JavaScript guest bindings:

```javascript
import { shareText, shareFile } from "@choochmeque/tauri-plugin-sharekit-api";

// Share text
await shareText('Tauri is great!');

// Share a file
await shareFile('file:///path/to/document.pdf', {
  mimeType: 'application/pdf',
  title: 'My Document'
});
```

### Receiving Shared Content (Share Target)

Your app can receive content shared from other apps:

```javascript
import {
  getPendingSharedContent,
  clearPendingSharedContent,
  onSharedContent
} from "@choochmeque/tauri-plugin-sharekit-api";

// Check for shared content on app startup (cold start)
const content = await getPendingSharedContent();
if (content) {
  if (content.type === 'text') {
    console.log('Received text:', content.text);
  } else if (content.type === 'files') {
    for (const file of content.files) {
      console.log('Received file:', file.name, file.path);
    }
  }
  await clearPendingSharedContent();
}

// Listen for shares while app is running (warm start)
const unlisten = await onSharedContent((content) => {
  console.log('Received:', content);
});
```

## Platform Setup

### Android

To receive shared content on Android, add intent filters to your `AndroidManifest.xml`:

`src-tauri/gen/android/app/src/main/AndroidManifest.xml`

Add these intent filters inside your `<activity>` tag:

```xml
<!-- Receive text shares -->
<intent-filter>
    <action android:name="android.intent.action.SEND" />
    <category android:name="android.intent.category.DEFAULT" />
    <data android:mimeType="text/*" />
</intent-filter>

<!-- Receive image shares -->
<intent-filter>
    <action android:name="android.intent.action.SEND" />
    <category android:name="android.intent.category.DEFAULT" />
    <data android:mimeType="image/*" />
</intent-filter>

<!-- Receive any file -->
<intent-filter>
    <action android:name="android.intent.action.SEND" />
    <category android:name="android.intent.category.DEFAULT" />
    <data android:mimeType="*/*" />
</intent-filter>

<!-- Receive multiple files -->
<intent-filter>
    <action android:name="android.intent.action.SEND_MULTIPLE" />
    <category android:name="android.intent.category.DEFAULT" />
    <data android:mimeType="*/*" />
</intent-filter>
```

You can customize the `mimeType` values to only accept specific file types.

### iOS

To receive shared content on iOS, you need to add a Share Extension. Use the `@choochmeque/tauri-apple-extensions` tool after initializing your iOS project:

```bash
# First, initialize the iOS project if you haven't already
tauri ios init

# Then add the Share Extension
npx @choochmeque/tauri-apple-extensions ios add share --plugin @choochmeque/tauri-plugin-sharekit-api
```

The setup tool will:
1. Create a Share Extension target in your Xcode project
2. Configure App Groups for communication between the extension and main app
3. Add a URL scheme for the extension to open your app

**After running the script, you must:**

1. Open the Xcode project (`src-tauri/gen/apple/*.xcodeproj`)
2. Select your Apple Developer Team for both targets:
   - Main app target (e.g., `myapp_iOS`)
   - Share Extension target (e.g., `myapp-ShareExtension`)
3. Enable the "App Groups" capability for **both** targets in Xcode
4. In Apple Developer Portal, create the App Group (e.g., `group.com.your.app`) and add it to both App IDs

**App Group Configuration:**

The extension and main app communicate via App Groups. The setup script uses `group.{your.bundle.id}` as the App Group identifier. Make sure this is configured in:
- Apple Developer Portal (create the App Group)
- Both App IDs (main app and extension)
- Xcode capabilities for both targets

### macOS

To receive shared content on macOS, you need to add a Share Extension. First, create the macOS Xcode project, then add the extension:

```bash
# First, create the macOS Xcode project
npx @choochmeque/tauri-macos-xcode init

# Then add the Share Extension
npx @choochmeque/tauri-apple-extensions macos add share --plugin @choochmeque/tauri-plugin-sharekit-api
```

The setup tool will:
1. Create a Share Extension target in your Xcode project
2. Configure App Groups for communication between the extension and main app
3. Add a URL scheme for the extension to open your app

**After running the script, you must:**

1. Open the Xcode project (`src-tauri/gen/apple-macos/*.xcodeproj`)
2. Select your Apple Developer Team for both targets:
   - Main app target (e.g., `myapp_macOS`)
   - Share Extension target (e.g., `myapp-ShareExtension`)
3. Enable the "App Groups" capability for **both** targets in Xcode
4. In Apple Developer Portal, create the App Group (e.g., `group.com.your.app`) and add it to both App IDs

**Development workflow:**

```bash
# Start the dev server and open Xcode
pnpm tauri:macos:dev

# Then press Cmd+R in Xcode to build and run
```

### Displaying Received Images

To display received images in your app, enable the asset protocol feature and configure the scope.

`src-tauri/Cargo.toml`

```toml
[dependencies]
tauri = { version = "2", features = ["protocol-asset"] }
```

`src-tauri/tauri.conf.json`

```json
{
  "app": {
    "security": {
      "assetProtocol": {
        "enable": true,
        "scope": [
          "$CACHE/**",
          "$APPCACHE/**",
          "**/Containers/Shared/AppGroup/**"
        ]
      }
    }
  }
}
```

The `**/Containers/Shared/AppGroup/**` scope is required on iOS to access files shared via the Share Extension (works for both simulator and real devices).

Then in your frontend:

```javascript
import { convertFileSrc } from "@tauri-apps/api/core";

// file.path is from SharedContent.files
const imageUrl = convertFileSrc(file.path);
// Use imageUrl in an <img> tag
```

## Contributing

PRs accepted. Please make sure to read the Contributing Guide before making a pull request.

## License

MIT
