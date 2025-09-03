# Tauri Plugin ShareKit

Share content to other apps via native Android and iOS sharing interfaces.

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
tauri-plugin-sharekit = "0.1"
# alternatively with Git:
tauri-plugin-sharekit = { git = "https://github.com/Choochmeque/tauri-plugin-sharekit" }
```

You can install the JavaScript Guest bindings using your preferred JavaScript package manager:

> Note: Since most JavaScript package managers are unable to install packages from git monorepos we provide read-only mirrors of each plugin. This makes installation option 2 more ergonomic to use.

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

## Contributing

PRs accepted. Please make sure to read the Contributing Guide before making a pull request.

## License

MIT