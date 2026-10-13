/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** "1" in development-mode builds; enables the tauri-plugin-mcp bridge. */
  readonly VITE_MCP_BRIDGE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
