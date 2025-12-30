<script lang="ts">
  import { onMount } from "svelte";
  import {
    shareText,
    shareFile,
    getPendingSharedContent,
    clearPendingSharedContent,
    onSharedContent,
    type SharedContent
  } from "@choochmeque/tauri-plugin-sharekit-api";

  // Text sharing state
  let text = $state("Hello from Tauri ShareKit!");
  let textMimeType = $state("");
  let textStatus = $state("");

  // File sharing state
  let fileUrl = $state("");
  let fileMimeType = $state("");
  let fileTitle = $state("");
  let fileStatus = $state("");

  // Received content state
  let receivedContent = $state<SharedContent | null>(null);

  onMount(async () => {
    // Check for content from cold start
    await checkForSharedContent();

    // Listen for content from warm start
    const listener = await onSharedContent((content) => {
      receivedContent = content;
    });

    return () => listener.unregister();
  });

  async function checkForSharedContent() {
    try {
      const content = await getPendingSharedContent();
      if (content) {
        receivedContent = content;
      }
    } catch (e) {
      console.error("Failed to get shared content:", e);
    }
  }

  async function handleClearSharedContent() {
    await clearPendingSharedContent();
    receivedContent = null;
  }

  async function handleShareText() {
    textStatus = "Sharing...";
    try {
      const options = textMimeType ? { mimeType: textMimeType } : undefined;
      await shareText(text, options);
      textStatus = "Shared successfully!";
    } catch (e) {
      textStatus = `Error: ${e}`;
    }
  }

  async function handleShareFile() {
    if (!fileUrl) {
      fileStatus = "Please enter a file path";
      return;
    }
    fileStatus = "Sharing...";
    try {
      const options: { mimeType?: string; title?: string } = {};
      if (fileMimeType) options.mimeType = fileMimeType;
      if (fileTitle) options.title = fileTitle;
      await shareFile(fileUrl, Object.keys(options).length ? options : undefined);
      fileStatus = "Shared successfully!";
    } catch (e) {
      fileStatus = `Error: ${e}`;
    }
  }
</script>

<main class="container">
  <h1>ShareKit Demo</h1>

  {#if receivedContent}
    <section class="card received">
      <h2>Received Content</h2>
      <p><strong>Type:</strong> {receivedContent.type}</p>
      {#if receivedContent.type === "text" && receivedContent.text}
        <p><strong>Text:</strong> {receivedContent.text}</p>
      {:else if receivedContent.type === "files" && receivedContent.files}
        <p><strong>Files:</strong></p>
        <ul>
          {#each receivedContent.files as file}
            <li>
              <strong>{file.name}</strong>
              <br /><small>Path: {file.path}</small>
              {#if file.mimeType}<br /><small>MIME: {file.mimeType}</small>{/if}
              {#if file.size}<br /><small>Size: {file.size} bytes</small>{/if}
            </li>
          {/each}
        </ul>
      {/if}
      <button onclick={handleClearSharedContent}>Clear</button>
    </section>
  {/if}

  <section class="card">
    <h2>Share Text</h2>
    <div class="form-group">
      <label for="text">Text to share</label>
      <textarea id="text" bind:value={text} rows="3"></textarea>
    </div>
    <div class="form-group">
      <label for="text-mime">MIME Type (optional, Android only)</label>
      <input id="text-mime" type="text" bind:value={textMimeType} placeholder="text/plain" />
    </div>
    <button onclick={handleShareText}>Share Text</button>
    {#if textStatus}
      <p class="status" class:error={textStatus.startsWith("Error")}>{textStatus}</p>
    {/if}
  </section>

  <section class="card">
    <h2>Share File</h2>
    <div class="form-group">
      <label for="file-url">File URL / Path</label>
      <input id="file-url" type="text" bind:value={fileUrl} placeholder="file:///path/to/file.pdf or /path/to/file.pdf" />
    </div>
    <div class="form-group">
      <label for="file-mime">MIME Type (optional)</label>
      <input id="file-mime" type="text" bind:value={fileMimeType} placeholder="application/pdf" />
    </div>
    <div class="form-group">
      <label for="file-title">Title (optional)</label>
      <input id="file-title" type="text" bind:value={fileTitle} placeholder="Document.pdf" />
    </div>
    <button onclick={handleShareFile}>Share File</button>
    {#if fileStatus}
      <p class="status" class:error={fileStatus.startsWith("Error")}>{fileStatus}</p>
    {/if}
  </section>
</main>

<style>
  :root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 24px;
    font-weight: 400;
    color: #0f0f0f;
    background-color: #f6f6f6;
    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  .container {
    max-width: 600px;
    margin: 0 auto;
    padding: 2rem;
  }

  h1 {
    text-align: center;
    margin-bottom: 2rem;
  }

  h2 {
    margin-top: 0;
    margin-bottom: 1rem;
    font-size: 1.25rem;
  }

  .card {
    background: white;
    border-radius: 8px;
    padding: 1.5rem;
    margin-bottom: 1.5rem;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .card.received {
    border: 2px solid #396cd8;
    background: #e8f0fe;
  }

  .card.received ul {
    margin: 0.5rem 0;
    padding-left: 1.5rem;
  }

  .card.received li {
    margin-bottom: 0.5rem;
    word-break: break-all;
  }

  .card.received p {
    word-break: break-all;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  label {
    display: block;
    margin-bottom: 0.25rem;
    font-weight: 500;
    font-size: 0.875rem;
  }

  input, textarea {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 1rem;
    box-sizing: border-box;
  }

  textarea {
    resize: vertical;
  }

  button {
    width: 100%;
    padding: 0.75rem;
    background: #396cd8;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s;
  }

  button:hover {
    background: #2a5cb8;
  }

  button:active {
    background: #1e4a9e;
  }

  .status {
    margin-top: 0.75rem;
    margin-bottom: 0;
    padding: 0.5rem;
    border-radius: 4px;
    background: #d4edda;
    color: #155724;
    font-size: 0.875rem;
  }

  .status.error {
    background: #f8d7da;
    color: #721c24;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }

    .card {
      background: #3f3f3f;
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    }

    .card.received {
      background: #2a3a5c;
      border-color: #5a8cfa;
    }

    input, textarea {
      background: #2f2f2f;
      border-color: #555;
      color: #f6f6f6;
    }

    .status {
      background: #1e4620;
      color: #a3d9a5;
    }

    .status.error {
      background: #4a1c1c;
      color: #f5a5a5;
    }
  }
</style>
