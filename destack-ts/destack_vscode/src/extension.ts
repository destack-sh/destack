import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import * as vscode from "vscode";
import {
  LanguageClient,
  type LanguageClientOptions,
  LogMessageNotification,
  MessageType,
  type ServerOptions,
  type StreamInfo,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let serverProc: ChildProcessWithoutNullStreams | undefined;

/**
 * Activate the Destack VSCode extension.
 * Sets up the language server client and establishes communication.
 */
export async function activate(ctx: vscode.ExtensionContext) {
  // create output channels for logging
  const clientLog = vscode.window.createOutputChannel("Destack Client", { log: true });
  const serverLog = vscode.window.createOutputChannel("Destack Language Server", { log: true });

  const cfg = vscode.workspace.getConfiguration("destack");
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];

  const serverOptions: ServerOptions = async (): Promise<StreamInfo> => {
    const serverCommand = cfg.get<string>("server.command") ?? "";
    const args = cfg.get<string[]>("server.args") ?? [];
    const cwd = cfg.get<string>("server.cwd") ?? "";

    if (!serverCommand) {
      serverLog.error("destack.server.command is required but not set");
      throw new Error("destack.server.command is required");
    }

    serverLog.info(`Using Destack LSP: ${serverCommand}`);

    return await new Promise<StreamInfo>((resolve, reject) => {
      serverProc = spawn(serverCommand, args, {
        stdio: ["pipe", "pipe", "pipe"],
        cwd: cwd || workspaceFolder?.uri.fsPath,
        env: process.env,
      });
      serverProc.once("error", (err) => {
        serverLog.error(`Failed to spawn ${serverCommand}: ${err.message}`);
        reject(err);
      });
      serverProc.once("spawn", () => {
        serverLog.info(
          `Spawned ${serverCommand} ${args.join(" ")} (pid ${serverProc?.pid ?? ""}) cwd=${cwd || workspaceFolder?.uri.fsPath}`,
        );
        resolve({ reader: serverProc!.stdout, writer: serverProc!.stdin });
      });
      serverProc.stderr.setEncoding("utf8");
      serverProc.stderr.on("data", (chunk: string) => serverLog.append(chunk));
      serverProc.on("exit", (code, signal) => {
        serverLog.warn(`${serverCommand} exited (code=${code}, signal=${signal ?? ""})`);
      });
    });
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ language: "destack-ds" }],
    outputChannel: clientLog,
    traceOutputChannel: clientLog,
  };

  client = new LanguageClient("destack", "Destack LSP", serverOptions, clientOptions);

  // start the language client
  await client.start();
  clientLog.info("Destack LSP client started.");

  // forward server log messages to the server output channel
  client.onNotification(LogMessageNotification.type, (p) => {
    switch (p.type) {
      case MessageType.Error:
        serverLog.error(`[server] ${p.message}`);
        break;
      case MessageType.Warning:
        serverLog.warn(`[server] ${p.message}`);
        break;
      case MessageType.Info:
        serverLog.info(`[server] ${p.message}`);
        break;
      default:
        serverLog.trace?.(`[server] ${p.message}`);
        break;
    }
  });

  // register disposables for cleanup
  ctx.subscriptions.push(
    clientLog,
    serverLog,
    new vscode.Disposable(() => {
      if (serverProc && !serverProc.killed) {
        try {
          serverProc.kill();
        } catch {
          // ignore kill errors
        }
      }
    }),
  );
}

/**
 * Deactivate the extension.
 * Stops the language client and cleans up resources.
 */
export async function deactivate() {
  try {
    await client?.stop();
  } finally {
    // NOTE: server process cleanup is handled by disposables
  }
}
