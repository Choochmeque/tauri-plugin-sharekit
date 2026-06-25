import {
  invoke,
  addPluginListener,
  PluginListener,
} from "@tauri-apps/api/core";

export interface SharePosition {
  x: number;
  y: number;
  /** macOS only: which edge the picker appears from */
  preferredEdge?: "top" | "bottom" | "left" | "right";
}

export interface ShareTextOptions {
  /** Android only */
  mimeType?: string;
  /** Position for the share sheet (iPad/macOS only) */
  position?: SharePosition;
}

export interface ShareFileOptions {
  mimeType?: string;
  title?: string;
  /** Position for the share sheet (iPad/macOS only) */
  position?: SharePosition;
}

export interface SharedFile {
  path: string;
  name: string;
  mimeType?: string;
  size?: number;
}

export interface SharedContent {
  type: "text" | "files";
  text?: string;
  files?: SharedFile[];
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

/**
 * Gets content shared to this app from other apps.
 * Call this on app startup to check if the app was launched via share.
 *
 * ```javascript
 * import { getPendingSharedContent, clearPendingSharedContent } from "@choochmeque/tauri-plugin-sharekit-api";
 *
 * const content = await getPendingSharedContent();
 * if (content) {
 *   if (content.type === 'text') {
 *     console.log('Received text:', content.text);
 *   } else if (content.type === 'files') {
 *     console.log('Received files:', content.files);
 *   }
 *   await clearPendingSharedContent();
 * }
 * ```
 * @returns The shared content, or null if no content was shared
 */
export async function getPendingSharedContent(): Promise<SharedContent | null> {
  return await invoke<SharedContent | null>(
    "plugin:sharekit|get_pending_shared_content",
  );
}

/**
 * Clears the pending shared content after it has been processed.
 * Call this after handling shared content to prevent it from being
 * returned again on subsequent calls to getPendingSharedContent().
 */
export async function clearPendingSharedContent(): Promise<void> {
  await invoke("plugin:sharekit|clear_pending_shared_content");
}

/**
 * Listens for content shared to this app while it's running.
 * Use this for warm start scenarios (app already running when share happens).
 *
 * ```javascript
 * import { onSharedContent } from "@choochmeque/tauri-plugin-sharekit-api";
 *
 * const unlisten = await onSharedContent((content) => {
 *   console.log('Received:', content);
 * });
 *
 * // Later, to stop listening:
 * unlisten.unregister();
 * ```
 */
export async function onSharedContent(
  handler: (content: SharedContent) => void,
): Promise<PluginListener> {
  return await addPluginListener("sharekit", "sharedContent", handler);
}
