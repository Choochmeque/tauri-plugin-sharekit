import { invoke } from "@tauri-apps/api/core";

export interface ShareTextOptions {
  // Android only
  mimeType?: string;
}

export interface ShareFileOptions {
  mimeType?: string;
  title?: string;
}

/**
 * Opens the native sharing interface to share the specified text.
 *
 * ```javascript
 * import { shareText } from "@choochmeque/tauri-plugin-sharekit-api";
 * await shareText('I am a shared message');
 * ```
 * @param text
 * @param options
 * @returns
 */
export async function shareText(
  text: string,
  options?: ShareTextOptions,
): Promise<void> {
  await invoke("plugin:sharekit|share_text", {
    text,
    ...options,
  });
}

/**
 * Opens the native sharing interface to share a file.
 *
 * ```javascript
 * import { shareFile } from "@choochmeque/tauri-plugin-sharekit-api";
 * await shareFile('file:///path/to/file.pdf', {
 *   mimeType: 'application/pdf',
 *   title: 'Document.pdf'
 * });
 * ```
 * @param url - The file URL to share (must be a file:// URL)
 * @param options - Optional settings including MIME type and title
 * @returns
 */
export async function shareFile(
  url: string,
  options?: ShareFileOptions,
): Promise<void> {
  await invoke("plugin:sharekit|share_file", {
    url,
    ...options,
  });
}
