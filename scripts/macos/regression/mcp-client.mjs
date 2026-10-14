// A minimal client for tauri-plugin-mcp's socket in debug builds of Shrimp:
// one JSON request per line, one JSON response per line, matched by id. On
// Windows it is a named pipe that takes no token.

import { existsSync, readFileSync } from "node:fs";
import { createConnection } from "node:net";

const DEFAULT_PATH =
  process.platform === "win32" ? String.raw`\\.\pipe\tmp\tauri-mcp.sock` : "/tmp/tauri-mcp.sock";

export class McpClient {
  #socket;
  #buffer = "";
  #pending = new Map();
  #nextId = 1;
  #token;

  static async connect(path = process.env.TAURI_MCP_IPC_PATH ?? DEFAULT_PATH) {
    const client = new McpClient();
    const token = `${path}.token`;
    client.#token = existsSync(token) ? readFileSync(token, "utf8").trim() : undefined;
    await new Promise((resolve, reject) => {
      client.#socket = createConnection(path, resolve);
      client.#socket.once("error", reject);
    });
    client.#socket.setEncoding("utf8");
    client.#socket.on("data", (chunk) => client.#receive(chunk));
    client.#socket.on("close", () => {
      for (const { reject } of client.#pending.values()) reject(new Error("socket closed"));
      client.#pending.clear();
    });
    return client;
  }

  #receive(chunk) {
    this.#buffer += chunk;
    let newline;
    while ((newline = this.#buffer.indexOf("\n")) >= 0) {
      const line = this.#buffer.slice(0, newline);
      this.#buffer = this.#buffer.slice(newline + 1);
      if (!line.trim()) continue;
      const response = JSON.parse(line);
      const waiter = this.#pending.get(response.id);
      if (!waiter) continue;
      this.#pending.delete(response.id);
      clearTimeout(waiter.timer);
      if (response.success) waiter.resolve(response.data);
      else waiter.reject(new Error(response.error ?? "request failed"));
    }
  }

  send(command, payload = {}, timeoutMs = 30_000) {
    const id = String(this.#nextId++);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new Error(`${command} timed out`));
      }, timeoutMs);
      this.#pending.set(id, { resolve, reject, timer });
      this.#socket.write(`${JSON.stringify({ command, payload, id, authToken: this.#token })}\n`);
    });
  }

  /** Calls a Shrimp IPC command and returns its parsed result. */
  async invoke(command, args = {}, { window = "main", timeoutMs = 20_000 } = {}) {
    const data = await this.send(
      "manage_ipc",
      { action: "invoke", command, args, window_label: window, timeout_ms: timeoutMs },
      timeoutMs + 5_000,
    );
    if (data?.error) throw new Error(`${command}: ${data.error}`);
    const result = data?.result;
    if (typeof result !== "string") return result ?? null;
    try {
      return JSON.parse(result);
    } catch {
      return result;
    }
  }

  /** Log entries after `sinceId` (Rust and webview), oldest first. */
  async logs(sinceId, { contains, limit = 1000 } = {}) {
    const payload = { mode: "tail", limit, head: true };
    if (sinceId !== undefined) payload.sinceId = sinceId;
    if (contains !== undefined) payload.contains = contains;
    return this.send("query_logs", payload);
  }

  /** Visible text of the elements matching `selector` in a webview. */
  async readText(selector, window = "main") {
    const data = await this.send("read_text", {
      selector,
      window_label: window,
      all: true,
      limit: 50,
    });
    return (data?.matches ?? []).map((element) => element.text ?? "");
  }

  close() {
    this.#socket?.end();
  }
}
