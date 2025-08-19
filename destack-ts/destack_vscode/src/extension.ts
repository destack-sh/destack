import * as path from "node:path";
import * as vscode from "vscode";
import { spawn, ChildProcessWithoutNullStreams } from "child_process";
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
  type StreamInfo,
  Trace,
  MessageType,
  LogMessageNotification,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let serverProc: ChildProcessWithoutNullStreams | undefined;

/**
 * Activate the Destack VSCode extension.
 * Sets up the LSP client and connects to the LSP server.
 */
export async function activate(context: vscode.ExtensionContext) {
  const clientChan = vscode.window.createOutputChannel("Destack Client");
  const serverChan = vscode.window.createOutputChannel("Destack Server");

  clientChan.appendLine("Destack activate()");
  serverChan.appendLine("Server channel ready");

  // determine server executable path from env or default location
  // (NOTE: we default to local internal development for now)
  const serverExecutable =
    process.env.DESTACK_LSP_PATH ||
    path.join(
      vscode.workspace.rootPath || "",
      "target",
      "debug",
      process.platform === "win32" ? "destack_lsp.exe" : "destack_lsp",
    );

  // spawn the server ourselves so we can tee its stderr to the server channel
  const serverOptions: ServerOptions = async (): Promise<StreamInfo> => {
    serverProc = spawn(serverExecutable, [], { stdio: ["pipe", "pipe", "pipe"] });

    serverChan.appendLine(`spawned destack_lsp (pid ${serverProc.pid})`);

    // forward server stderr verbatim
    serverProc.stderr.setEncoding("utf8");
    serverProc.stderr.on("data", (chunk: string) => {
      serverChan.append(chunk); // already contains newlines typically
    });

    serverProc.on("exit", (code, signal) => {
      serverChan.appendLine(`destack_lsp exited (code=${code} signal=${signal ?? ""})`);
    });

    // IMPORTANT: don't touch stdout; it's the LSP protocol stream
    return {
      reader: serverProc.stdout,
      writer: serverProc.stdin,
    };
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ language: "destack-ds" }],
    // send client logs and protocol traces here
    outputChannel: clientChan,
    traceOutputChannel: clientChan,
  };

  client = new LanguageClient("destack", "Destack LSP", serverOptions, clientOptions);
  client.trace = Trace.Verbose;

  // when the client is ready, also forward window/logMessage to the server channel
  client.onReady().then(() => {
    serverChan.appendLine("LSP client ready; forwarding window/logMessage → Destack Server");
    client!.onNotification(LogMessageNotification.type, (p) => {
      const level =
        p.type === MessageType.Error
          ? "ERROR"
          : p.type === MessageType.Warning
            ? "WARN"
            : p.type === MessageType.Info
              ? "INFO"
              : "LOG";
      serverChan.appendLine(`[${level}] ${p.message}`);
    });
  });

  context.subscriptions.push(
    client.start(),
    clientChan,
    serverChan,
    new vscode.Disposable(() => {
      if (serverProc && !serverProc.killed) {
        try {
          serverProc.kill();
        } catch {
          /* ignore */
        }
      }
    }),
  );
}

/**
 * Deactivate the extension and stop the language client.
 */
export async function deactivate() {
  try {
    await client?.stop();
  } finally {
    if (serverProc && !serverProc.killed) {
      try {
        serverProc.kill();
      } catch {
        /* ignore */
      }
    }
  }
}
