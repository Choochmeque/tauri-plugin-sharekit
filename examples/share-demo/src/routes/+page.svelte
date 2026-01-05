<script lang="ts">
  import { shareText, shareFile, type SharePosition } from "@choochmeque/tauri-plugin-sharekit-api";

  // Text sharing state
  let text = $state("Hello from Tauri ShareKit!");
  let textMimeType = $state("");
  let textStatus = $state("");

  // File sharing state
  let fileUrl = $state("");
  let fileMimeType = $state("");
  let fileTitle = $state("");
  let fileStatus = $state("");

  // Position state (iPad/macOS only)
  let usePosition = $state(false);
  let preferredEdge = $state<"top" | "bottom" | "left" | "right">("bottom");

  async function handleShareText(event: MouseEvent) {
    textStatus = "Sharing...";
    try {
      const position = usePosition
        ? { x: event.clientX, y: event.clientY, preferredEdge }
        : undefined;
      const options = textMimeType || position
        ? { mimeType: textMimeType || undefined, position }
        : undefined;
      await shareText(text, options);
      textStatus = "Shared successfully!";
    } catch (e) {
      textStatus = `Error: ${e}`;
    }
  }

  async function handleShareFile(event: MouseEvent) {
    if (!fileUrl) {
      fileStatus = "Please enter a file path";
      return;
    }
    fileStatus = "Sharing...";
    try {
      const position = usePosition
        ? { x: event.clientX, y: event.clientY, preferredEdge }
        : undefined;
      const options: { mimeType?: string; title?: string; position?: SharePosition } = {};
      if (fileMimeType) options.mimeType = fileMimeType;
      if (fileTitle) options.title = fileTitle;
      if (position) options.position = position;
      await shareFile(fileUrl, Object.keys(options).length ? options : undefined);
      fileStatus = "Shared successfully!";
    } catch (e) {
      fileStatus = `Error: ${e}`;
    }
  }
</script>

<main class="container">
  <h1>ShareKit Demo</h1>

  <section class="card">
    <h2>Position Settings (iPad/macOS)</h2>
    <div class="form-group checkbox-group">
      <label>
        <input type="checkbox" bind:checked={usePosition} />
        Position share sheet at click location
      </label>
    </div>
    {#if usePosition}
      <div class="form-group">
        <label for="edge">Preferred Edge (macOS only)</label>
        <select id="edge" bind:value={preferredEdge}>
          <option value="top">Top</option>
          <option value="bottom">Bottom</option>
          <option value="left">Left</option>
          <option value="right">Right</option>
        </select>
      </div>
    {/if}
  </section>

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
    <button onclick={(e) => handleShareText(e)}>Share Text</button>
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
    <button onclick={(e) => handleShareFile(e)}>Share File</button>
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

  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 1rem;
    box-sizing: border-box;
    background: white;
  }

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .checkbox-group input[type="checkbox"] {
    width: auto;
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

    input, textarea, select {
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
