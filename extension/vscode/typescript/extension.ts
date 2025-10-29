import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { PassThrough } from "node:stream";
import * as vscode from "vscode";
import {
    LanguageClient,
    type LanguageClientOptions,
    LogMessageNotification,
    MessageType,
    type ServerOptions,
    State,
    type StreamInfo,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let serverProc: ChildProcessWithoutNullStreams | undefined;

const DEBUG = false;



/**
 * Try to stop the server process gracefully, then force-kill if needed.
 */
async function stopServerProc(serverLog: vscode.OutputChannel) {
    const proc = serverProc;
    if (!proc || proc.killed) return;

    serverProc = undefined;

    try {
        // Close stdin to encourage graceful shutdown.
        proc.stdin.end();
    } catch {}

    try {
        process.kill(proc.pid!, "SIGTERM");
    } catch {}

    const exited = await new Promise<boolean>((resolve) => {
        const to = setTimeout(() => resolve(false), 1500);
        proc.once("exit", () => {
            clearTimeout(to);
            resolve(true);
        });
        proc.once("close", () => {
            clearTimeout(to);
            resolve(true);
        });
    });

    if (!exited) {
        serverLog.appendLine("Server didn't exit on SIGTERM, sending SIGKILL.");
        try {
            process.kill(proc.pid!, "SIGKILL");
        } catch {}
    }
}

/**
 * Activate the Destack VSC ode extension.
 * Sets up the language server client and establishes communication.
 */
export async function activate(ctx: vscode.ExtensionContext) {
    const clientLog = vscode.window.createOutputChannel("Destack Client", { log: true });
    const serverLog = vscode.window.createOutputChannel("Destack Server", { log: true });
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

        serverLog.info(`Using Destack Server: ${serverCommand} ${args.join(" ")}`);

        return await new Promise<StreamInfo>((resolve, reject) => {
            // prepare env
            const env = { ...process.env };
            if (DEBUG) {
                env.WAIT_FOR_DEBUGGER = "1";
                env.RUST_LOG = env.RUST_LOG || "trace";
                env.RUST_BACKTRACE = env.RUST_BACKTRACE || "full";
            }

            // spawn and assign the module-level serverProc
            serverProc = spawn(serverCommand, args, {
                stdio: ["pipe", "pipe", "pipe"],
                cwd: cwd || workspaceFolder?.uri.fsPath,
                env,
                shell: false,
            });

            // stderr -> serverLog
            serverProc.stderr.setEncoding("utf8");
            serverProc.stderr.on("data", (chunk: string) => serverLog.append(chunk));

            serverProc.once("spawn", () => {
                serverLog.info(
                    `Spawned ${serverCommand} ${args.join(" ")} (pid ${serverProc?.pid ?? ""}) cwd=${cwd || workspaceFolder?.uri.fsPath}`,
                );

                if (DEBUG) {
                    // Tee server stdout to both client reader and log.
                    const outTee = new PassThrough();
                    serverProc!.stdout.pipe(outTee);
                    outTee.on("data", (chunk) => {
                        // Log raw LSP from server -> client
                        serverLog.append(`[server → client]\n${chunk.toString()}\n`);
                    });

                    // Tee client writer to both child stdin and log.
                    const inTee = new PassThrough();
                    inTee.on("data", (chunk) => {
                        // Log raw LSP from client -> server
                        serverLog.append(`[client → server]\n${chunk.toString()}\n`);
                    });
                    // **IMPORTANT**: pipe the tee into the real stdin
                    inTee.pipe(serverProc!.stdin);

                    resolve({ reader: outTee, writer: inTee });
                } else {
                    resolve({ reader: serverProc!.stdout, writer: serverProc!.stdin });
                }
            });

            serverProc.once("error", (err) => {
                serverLog.error(`Failed to spawn ${serverCommand}: ${err.message}`);
                reject(err);
            });

            serverProc.on("exit", (code, signal) => {
                serverLog.warn(`${serverCommand} exited (code=${code}, signal=${signal ?? ""})`);
            });
        });
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { language: "dyst" },
            { language: "dst" },
            { language: "dsb" },
            { language: "dsx" },
            { pattern: "**/*.ds" },
            { pattern: "**/*.dst" },
            { pattern: "**/*.dsb" },
            { pattern: "**/*.dsx" },
        ],
        outputChannel: clientLog,
        traceOutputChannel: clientLog,
    };

    client = new LanguageClient("destack", "Destack", serverOptions, clientOptions);

    // When client fully stops, make sure the child is gone.
    client.onDidChangeState((e) => {
        // 2 === Stopped
        if (e.newState == State.Stopped) {
            stopServerProc(serverLog);
        }
    });

    // start the language client
    await client.start();
    clientLog.info("Destack client started.");

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
            case MessageType.Log:
                serverLog.info(`[server] ${p.message}`);
                break;
            case MessageType.Debug:
                (serverLog as any).debug?.(`[server] ${p.message}`);
                break;
            default:
                (serverLog as any).trace?.(`[server] ${p.message}`);
                break;
        }
    });

    // register disposables for cleanup
    ctx.subscriptions.push(
        clientLog,
        serverLog,
        new vscode.Disposable(() => {
            // Final guard on extension deactivation/disposal.
            void stopServerProc(serverLog);
        }),
    );

    // restart the language server
    ctx.subscriptions.push(
        vscode.commands.registerCommand("destack.restart", async () => {
            try {
                // Ensure the previous child is gone before starting anew.
                await client!.stop();
                await stopServerProc(serverLog);
                await client!.start();
                vscode.window.showInformationMessage(`Destack restarted.`);
            } catch (e: any) {
                vscode.window.showErrorMessage(`Destack restart failed: ${e?.message || e}`);
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
        // Ensure server process is terminated.
        const serverLog = vscode.window.createOutputChannel("Destack Server", { log: true });
        await stopServerProc(serverLog);
        serverLog.dispose();
    }
}
