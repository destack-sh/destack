import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
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
let clientLogOutput: vscode.LogOutputChannel | undefined;
let serverLogOutput: vscode.LogOutputChannel | undefined;
let isConfigRestartInFlight = false;

const DEBUG = false;
const FALLBACK_COMMANDS = ["destack", "ds", "dsc"];
const SERVER_SETTING_KEYS = [
    "destack.server.command",
    "destack.server.args",
    "destack.server.cwd",
];
const LIVE_SETTING_KEYS = [
    "destack.completion.autoImports",
    "destack.inlayHints.parameterHints",
    "destack.inlayHints.typeHints",
];

type ResolvedServerCommand = {
    command: string;
    args: string[];
    cwd?: string;
};

function activeWorkspaceFolder(): vscode.WorkspaceFolder | undefined {
    const activeUri = vscode.window.activeTextEditor?.document.uri;
    if (activeUri) {
        const folder = vscode.workspace.getWorkspaceFolder(activeUri);
        if (folder) {
            return folder;
        }
    }

    return vscode.workspace.workspaceFolders?.[0];
}

function shouldRestartForConfigurationChange(event: vscode.ConfigurationChangeEvent): boolean {
    return SERVER_SETTING_KEYS.some((settingKey) => event.affectsConfiguration(settingKey));
}

function shouldNotifyForConfigurationChange(event: vscode.ConfigurationChangeEvent): boolean {
    return LIVE_SETTING_KEYS.some((settingKey) => event.affectsConfiguration(settingKey));
}

function expandPath(value: string, workspaceFolder?: vscode.WorkspaceFolder): string {
    if (!value) return value;

    let expanded = value;
    if (workspaceFolder) {
        expanded = expanded.replace(/\$\{workspaceFolder\}/g, workspaceFolder.uri.fsPath);
    }

    if (expanded == "~") {
        return os.homedir();
    }

    if (expanded.startsWith(`~${path.sep}`)) {
        return path.join(os.homedir(), expanded.slice(2));
    }

    return expanded;
}

function resolveOnPath(command: string): string | undefined {
    const pathEnv = process.env.PATH || "";
    const pathExts =
        process.platform == "win32"
            ? (process.env.PATHEXT || ".EXE;.CMD;.BAT;.COM").split(";")
            : [""];

    for (const base of pathEnv.split(path.delimiter)) {
        if (!base) continue;
        for (const ext of pathExts) {
            const candidate = path.join(base, `${command}${ext}`);
            if (fs.existsSync(candidate)) {
                return candidate;
            }
        }
    }

    return undefined;
}

function resolveCommandPath(command: string): string | undefined {
    if (!command) return undefined;

    if (path.isAbsolute(command) || command.includes(path.sep)) {
        return command;
    }

    return resolveOnPath(command);
}

function shouldInjectLsp(command: string, args: string[]): boolean {
    const base = path.basename(command).toLowerCase();
    const normalized = base.endsWith(".exe") ? base.slice(0, -4) : base;
    if (!FALLBACK_COMMANDS.includes(normalized)) {
        return false;
    }

    return args.length == 0 || args[0] != "lsp";
}

function resolveServerCommand(
    cfg: vscode.WorkspaceConfiguration,
    workspaceFolder: vscode.WorkspaceFolder | undefined,
): ResolvedServerCommand {
    const rawCommand = cfg.get<string>("server.command") ?? "";
    const rawArgs = cfg.get<string[]>("server.args") ?? [];
    const rawCwd = cfg.get<string>("server.cwd") ?? "";
    const cwd = rawCwd ? expandPath(rawCwd, workspaceFolder) : workspaceFolder?.uri.fsPath;

    if (rawCommand) {
        const expanded = expandPath(rawCommand, workspaceFolder);
        const resolved = resolveCommandPath(expanded);
        if (!resolved && (path.isAbsolute(expanded) || expanded.includes(path.sep))) {
            throw new Error(`destack.server.command not found: ${expanded}`);
        }

        const command = resolved ?? expanded;
        const args = shouldInjectLsp(command, rawArgs) ? ["lsp", ...rawArgs] : rawArgs;
        return { command, args, cwd };
    }

    for (const fallback of FALLBACK_COMMANDS) {
        const resolved = resolveOnPath(fallback);
        if (resolved) {
            return { command: resolved, args: ["lsp", ...rawArgs], cwd };
        }
    }

    const cargo = resolveOnPath("cargo");
    const cargoRoot = workspaceFolder?.uri.fsPath;
    if (cargo && cargoRoot) {
        const cargoToml = path.join(cargoRoot, "Cargo.toml");
        if (fs.existsSync(cargoToml)) {
            return {
                command: cargo,
                args: [
                    "run",
                    "-q",
                    "-p",
                    "destack_cli",
                    "--bin",
                    "destack",
                    "--",
                    "lsp",
                    ...rawArgs,
                ],
                cwd: cargoRoot,
            };
        }
    }

    throw new Error(
        "destack.server.command is unset and no Destack CLI found on PATH (destack, ds, dsc).",
    );
}

/**
 * Try to stop the server process gracefully, then force-kill if needed.
 */
async function stopServerProc(serverLog: vscode.OutputChannel) {
    const proc = serverProc;
    if (!proc || proc.killed) return;

    serverProc = undefined;

    try {
        // close stdin to encourage graceful shutdown
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
        serverLog.appendLine("server didn't exit on SIGTERM, sending SIGKILL.");
        try {
            process.kill(proc.pid!, "SIGKILL");
        } catch {}
    }
}

async function restartLanguageClient(
    status: vscode.StatusBarItem,
    serverLog: vscode.OutputChannel,
    successMessage: string,
) {
    const languageClient = client;
    if (!languageClient) {
        return;
    }

    status.text = "Destack: Restarting";
    status.tooltip = "Destack language server is restarting";

    // fully stop before starting again
    await languageClient.stop();
    await stopServerProc(serverLog);
    await languageClient.start();

    vscode.window.showInformationMessage(successMessage);
}

async function executeServerCommand(
    status: vscode.StatusBarItem,
    actionLabel: string,
    command: string,
    successMessage: string,
    failurePrefix: string,
) {
    const languageClient = client;
    if (!languageClient) {
        vscode.window.showErrorMessage("Destack server is not running.");
        return;
    }

    status.text = `Destack: ${actionLabel}`;
    try {
        await languageClient.sendRequest("workspace/executeCommand", {
            command,
            arguments: [],
        });
        status.text = "Destack: Ready";
        vscode.window.showInformationMessage(successMessage);
    } catch (error: any) {
        status.text = "Destack: Ready";
        vscode.window.showErrorMessage(`${failurePrefix}: ${error?.message || error}`);
    }
}

/**
 * Activate the Destack VSCode extension.
 * Sets up the language server client and establishes communication.
 */
export async function activate(ctx: vscode.ExtensionContext) {
    const clientLog = vscode.window.createOutputChannel("Destack Client", { log: true });
    const serverLog = vscode.window.createOutputChannel("Destack Server", { log: true });
    clientLogOutput = clientLog;
    serverLogOutput = serverLog;
    const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    status.command = "destack.restart";
    status.text = "Destack: Starting";
    status.tooltip = "Destack language server";
    status.show();

    const serverOptions: ServerOptions = async (): Promise<StreamInfo> => {
        const cfg = vscode.workspace.getConfiguration("destack");
        const workspaceFolder = activeWorkspaceFolder();

        let resolved: ResolvedServerCommand;
        try {
            resolved = resolveServerCommand(cfg, workspaceFolder);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            serverLog.error(message);
            throw new Error(message);
        }

        const { command: serverCommand, args, cwd } = resolved;
        serverLog.info(`using Destack: ${serverCommand} ${args.join(" ")}`);

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
                cwd,
                env,
                shell: false,
            });

            // stderr -> serverLog
            serverProc.stderr.setEncoding("utf8");
            serverProc.stderr.on("data", (chunk: string) => serverLog.append(chunk));

            serverProc.once("spawn", () => {
                serverLog.info(
                    `spawned ${serverCommand} ${args.join(" ")} (pid ${serverProc?.pid ?? ""}) cwd=${cwd || workspaceFolder?.uri.fsPath}`,
                );

                if (DEBUG) {
                    // tee server stdout to both client reader and log
                    const outTee = new PassThrough();
                    serverProc!.stdout.pipe(outTee);
                    outTee.on("data", (chunk) => {
                        serverLog.append(`[server → client]\n${chunk.toString()}\n`);
                    });

                    // tee client writer to both child stdin and log
                    const inTee = new PassThrough();
                    inTee.on("data", (chunk) => {
                        serverLog.append(`[client → server]\n${chunk.toString()}\n`);
                    });
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
            { language: "destack" },
            { language: "dst" },
            { language: "dsb" },
            { pattern: "**/*.ds" },
            { pattern: "**/*.d.ds" },
            { pattern: "**/*.dst" },
            { pattern: "**/*.dsb" },
        ],
        outputChannel: clientLog,
        traceOutputChannel: clientLog,
    };

    client = new LanguageClient("destack", "Destack", serverOptions, clientOptions);

    // when the client fully stops, make sure the child is gone
    client.onDidChangeState((e) => {
        switch (e.newState) {
            case State.Starting:
                status.text = "Destack: Starting";
                status.tooltip = "Destack language server is starting";
                break;
            case State.Running:
                status.text = "Destack: Ready";
                status.tooltip = "Destack language server is running";
                break;
            case State.Stopped:
                status.text = "Destack: Stopped";
                status.tooltip = "Destack language server is stopped";
                break;
        }

        // 2 === stopped
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
        status,
        new vscode.Disposable(() => {
            // final guard on extension deactivation and disposal
            void stopServerProc(serverLog);
        }),
    );

    // restart the language server
    ctx.subscriptions.push(
        vscode.commands.registerCommand("destack.restart", async () => {
            try {
                await restartLanguageClient(status, serverLog, "Destack restarted.");
            } catch (e: any) {
                vscode.window.showErrorMessage(`Destack restart failed: ${e?.message || e}`);
            }
        }),
    );

    ctx.subscriptions.push(
        vscode.commands.registerCommand("destack.rescan", async () => {
            await executeServerCommand(
                status,
                "Rescanning",
                "destack.rescan",
                "Destack rescan completed.",
                "Destack rescan failed",
            );
        }),
    );

    ctx.subscriptions.push(
        vscode.commands.registerCommand("destack.reindex", async () => {
            await executeServerCommand(
                status,
                "Reindexing",
                "destack.reindex",
                "Destack reindex completed.",
                "Destack reindex failed",
            );
        }),
    );

    ctx.subscriptions.push(
        vscode.commands.registerCommand("destack.clearCache", async () => {
            await executeServerCommand(
                status,
                "Clearing Cache",
                "destack.clearCache",
                "Destack cache cleared.",
                "Destack cache clear failed",
            );
        }),
    );

    ctx.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(async (event) => {
            // notify the server about live setting updates
            if (shouldNotifyForConfigurationChange(event)) {
                try {
                    await client?.sendNotification("workspace/didChangeConfiguration", {
                        settings: {},
                    });
                } catch (error: any) {
                    vscode.window.showErrorMessage(
                        `Destack configuration update failed: ${error?.message || error}`,
                    );
                }
            }

            // restart only when process launch settings changed
            if (!shouldRestartForConfigurationChange(event) || isConfigRestartInFlight) {
                return;
            }

            isConfigRestartInFlight = true;
            try {
                await restartLanguageClient(status, serverLog, "Destack restarted for settings.");
            } catch (error: any) {
                vscode.window.showErrorMessage(
                    `Destack restart after settings change failed: ${error?.message || error}`,
                );
            } finally {
                isConfigRestartInFlight = false;
            }
        }),
    );

    ctx.subscriptions.push(
        vscode.commands.registerCommand("destack.showClientLogs", () => {
            clientLog.show(true);
        }),
    );

    ctx.subscriptions.push(
        vscode.commands.registerCommand("destack.showServerLogs", () => {
            serverLog.show(true);
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
        // ensure server process is terminated
        const serverLog = serverLogOutput;
        if (serverLog) {
            await stopServerProc(serverLog);
        }

        client = undefined;
        serverProc = undefined;
        clientLogOutput = undefined;
        serverLogOutput = undefined;
    }
}
