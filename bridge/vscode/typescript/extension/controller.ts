import { type ChildProcessWithoutNullStreams } from "node:child_process";
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

import { DEBUG } from "./constants";
import {
    getActiveWorkspaceFolder,
    isLiveConfigurationChange,
    isServerConfigurationChange,
    resolveServerCommand,
} from "./config";
import { spawnServerProcess, stopServerProcess } from "./process";

/**
 * The extension controller that owns VSCode client lifecycle and commands.
 */
export class DestackExtensionController {
    /**
     * The active language client.
     */
    private languageClient: LanguageClient | undefined;

    /**
     * The active lsp process.
     */
    private languageServerProcess: ChildProcessWithoutNullStreams | undefined;

    /**
     * The client log output channel.
     */
    private clientLogOutput: vscode.LogOutputChannel | undefined;

    /**
     * The server log output channel.
     */
    private serverLogOutput: vscode.LogOutputChannel | undefined;

    /**
     * The status bar indicator for extension state.
     */
    private statusBarItem: vscode.StatusBarItem | undefined;

    /**
     * The guard to prevent restart loops during config changes.
     */
    private isConfigurationRestartInFlight = false;

    /**
     * Activate the extension runtime and start the language client.
     */
    async activate(context: vscode.ExtensionContext): Promise<void> {
        // create output channels and status ui
        const clientLog = vscode.window.createOutputChannel("Destack Client", { log: true });
        const serverLog = vscode.window.createOutputChannel("Destack Server", { log: true });
        const statusBarItem = this.createStatusBarItem();

        // store extension runtime state
        this.clientLogOutput = clientLog;
        this.serverLogOutput = serverLog;
        this.statusBarItem = statusBarItem;

        // create the language client
        const serverOptions = this.createServerOptions(serverLog);
        const clientOptions = this.createClientOptions(clientLog);
        const languageClient = new LanguageClient(
            "destack",
            "Destack",
            serverOptions,
            clientOptions,
        );
        this.languageClient = languageClient;

        // wire state and logging listeners
        this.registerClientStateListener(languageClient, serverLog, statusBarItem);
        this.registerClientLogForwarding(languageClient, serverLog);

        // register core disposables
        context.subscriptions.push(clientLog, serverLog, statusBarItem);

        // start lsp and then register runtime commands/watchers
        await languageClient.start();
        clientLog.info("Destack client started.");

        this.registerCommands(context, statusBarItem, serverLog, clientLog);
        this.registerConfigurationWatcher(context, statusBarItem, serverLog);
        this.registerShutdownGuard(context, serverLog);
    }

    /**
     * Deactivate the extension runtime and stop child processes.
     */
    async deactivate(): Promise<void> {
        try {
            await this.languageClient?.stop();
        } finally {
            // stop the language server process if still alive
            const serverLog = this.serverLogOutput;
            if (serverLog) {
                await stopServerProcess(this.languageServerProcess, serverLog);
            }

            // clear controller runtime state
            this.languageClient = undefined;
            this.languageServerProcess = undefined;
            this.clientLogOutput = undefined;
            this.serverLogOutput = undefined;
            this.statusBarItem = undefined;
            this.isConfigurationRestartInFlight = false;
        }
    }

    /**
     * Create and configure the status bar item.
     */
    private createStatusBarItem(): vscode.StatusBarItem {
        const statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            100,
        );
        statusBarItem.command = "destack.restart";
        statusBarItem.text = "Destack: Starting";
        statusBarItem.tooltip = "Destack language server";
        statusBarItem.show();

        return statusBarItem;
    }

    /**
     * Create language client server options.
     */
    private createServerOptions(serverLog: vscode.LogOutputChannel): ServerOptions {
        return async (): Promise<StreamInfo> => {
            // resolve launch command from workspace settings
            const configuration = vscode.workspace.getConfiguration("destack");
            const workspaceFolder = getActiveWorkspaceFolder();

            let resolvedServerCommand;
            try {
                resolvedServerCommand = resolveServerCommand(configuration, workspaceFolder);
            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                serverLog.error(errorMessage);
                throw new Error(errorMessage);
            }

            // spawn lsp process and capture stream transports
            const { command, args, cwd } = resolvedServerCommand;
            serverLog.info(`using Destack: ${command} ${args.join(" ")}`);

            const spawnedServer = await spawnServerProcess({
                command,
                args,
                cwd,
                workspaceRoot: workspaceFolder?.uri.fsPath,
                serverLog,
                debug: DEBUG,
            });
            this.languageServerProcess = spawnedServer.process;

            return spawnedServer.streamInfo;
        };
    }

    /**
     * Create language client options.
     */
    private createClientOptions(clientLog: vscode.LogOutputChannel): LanguageClientOptions {
        return {
            documentSelector: [
                { language: "destack" },
                { language: "javascript" },
                { language: "javascriptreact" },
                { language: "typescript" },
                { language: "typescriptreact" },
                { pattern: "**/*.ds" },
                { pattern: "**/*.d.ds" },
            ],
            outputChannel: clientLog,
            traceOutputChannel: clientLog,
        };
    }

    /**
     * Register language client state updates.
     */
    private registerClientStateListener(
        languageClient: LanguageClient,
        serverLog: vscode.OutputChannel,
        statusBarItem: vscode.StatusBarItem,
    ): void {
        languageClient.onDidChangeState((event) => {
            this.updateStatusBarFromState(event.newState, statusBarItem);

            // ensure server process is stopped when client reaches stopped state
            if (event.newState == State.Stopped) {
                const languageServerProcess = this.languageServerProcess;
                this.languageServerProcess = undefined;
                void stopServerProcess(languageServerProcess, serverLog);
            }
        });
    }

    /**
     * Update status bar text from language client state.
     */
    private updateStatusBarFromState(
        state: State,
        statusBarItem: vscode.StatusBarItem,
    ): void {
        if (state == State.Starting) {
            statusBarItem.text = "Destack: Starting";
            statusBarItem.tooltip = "Destack language server is starting";
            return;
        }

        if (state == State.Running) {
            statusBarItem.text = "Destack: Ready";
            statusBarItem.tooltip = "Destack language server is running";
            return;
        }

        statusBarItem.text = "Destack: Stopped";
        statusBarItem.tooltip = "Destack language server is stopped";
    }

    /**
     * Register forwarding for server log notifications.
     */
    private registerClientLogForwarding(
        languageClient: LanguageClient,
        serverLog: vscode.LogOutputChannel,
    ): void {
        languageClient.onNotification(LogMessageNotification.type, (params) => {
            // map lsp log message types to vscode log levels
            if (params.type == MessageType.Error) {
                serverLog.error(`[server] ${params.message}`);
                return;
            }

            if (params.type == MessageType.Warning) {
                serverLog.warn(`[server] ${params.message}`);
                return;
            }

            if (params.type == MessageType.Info || params.type == MessageType.Log) {
                serverLog.info(`[server] ${params.message}`);
                return;
            }

            if (params.type == MessageType.Debug) {
                (serverLog as any).debug?.(`[server] ${params.message}`);
                return;
            }

            (serverLog as any).trace?.(`[server] ${params.message}`);
        });
    }

    /**
     * Register extension commands.
     */
    private registerCommands(
        context: vscode.ExtensionContext,
        statusBarItem: vscode.StatusBarItem,
        serverLog: vscode.LogOutputChannel,
        clientLog: vscode.LogOutputChannel,
    ): void {
        // register restart command
        context.subscriptions.push(
            vscode.commands.registerCommand("destack.restart", async () => {
                try {
                    await this.restartLanguageClient(
                        statusBarItem,
                        serverLog,
                        "Destack restarted.",
                    );
                } catch (error: any) {
                    const errorMessage = error?.message || error;
                    vscode.window.showErrorMessage(`Destack restart failed: ${errorMessage}`);
                }
            }),
        );

        // register server operation commands
        context.subscriptions.push(
            vscode.commands.registerCommand("destack.rescan", async () => {
                await this.executeServerCommand(
                    statusBarItem,
                    "Rescanning",
                    "destack.rescan",
                    "Destack rescan completed.",
                    "Destack rescan failed",
                );
            }),
        );
        context.subscriptions.push(
            vscode.commands.registerCommand("destack.reindex", async () => {
                await this.executeServerCommand(
                    statusBarItem,
                    "Reindexing",
                    "destack.reindex",
                    "Destack reindex completed.",
                    "Destack reindex failed",
                );
            }),
        );
        context.subscriptions.push(
            vscode.commands.registerCommand("destack.clearCache", async () => {
                await this.executeServerCommand(
                    statusBarItem,
                    "Clearing Cache",
                    "destack.clearCache",
                    "Destack cache cleared.",
                    "Destack cache clear failed",
                );
            }),
        );

        // register log reveal commands
        context.subscriptions.push(
            vscode.commands.registerCommand("destack.showClientLogs", () => {
                clientLog.show(true);
            }),
        );
        context.subscriptions.push(
            vscode.commands.registerCommand("destack.showServerLogs", () => {
                serverLog.show(true);
            }),
        );
    }

    /**
     * Register configuration change handling.
     */
    private registerConfigurationWatcher(
        context: vscode.ExtensionContext,
        statusBarItem: vscode.StatusBarItem,
        serverLog: vscode.LogOutputChannel,
    ): void {
        context.subscriptions.push(
            vscode.workspace.onDidChangeConfiguration(async (event) => {
                // push live setting changes without restart
                if (isLiveConfigurationChange(event)) {
                    await this.pushLiveConfiguration();
                }

                // restart only for server launch setting changes
                if (!isServerConfigurationChange(event)) {
                    return;
                }
                if (this.isConfigurationRestartInFlight) {
                    return;
                }

                this.isConfigurationRestartInFlight = true;
                try {
                    await this.restartLanguageClient(
                        statusBarItem,
                        serverLog,
                        "Destack restarted for settings.",
                    );
                } catch (error: any) {
                    const errorMessage = error?.message || error;
                    vscode.window.showErrorMessage(
                        `Destack restart after settings change failed: ${errorMessage}`,
                    );
                } finally {
                    this.isConfigurationRestartInFlight = false;
                }
            }),
        );
    }

    /**
     * Register final shutdown cleanup.
     */
    private registerShutdownGuard(
        context: vscode.ExtensionContext,
        serverLog: vscode.LogOutputChannel,
    ): void {
        context.subscriptions.push(
            new vscode.Disposable(() => {
                const languageServerProcess = this.languageServerProcess;
                this.languageServerProcess = undefined;
                void stopServerProcess(languageServerProcess, serverLog);
            }),
        );
    }

    /**
     * Push live configuration updates to the language server.
     */
    private async pushLiveConfiguration(): Promise<void> {
        const languageClient = this.languageClient;
        if (!languageClient) {
            return;
        }

        try {
            await languageClient.sendNotification("workspace/didChangeConfiguration", {
                settings: {},
            });
        } catch (error: any) {
            const errorMessage = error?.message || error;
            vscode.window.showErrorMessage(
                `Destack configuration update failed: ${errorMessage}`,
            );
        }
    }

    /**
     * Restart the language client and its backing server process.
     */
    private async restartLanguageClient(
        statusBarItem: vscode.StatusBarItem,
        serverLog: vscode.LogOutputChannel,
        successMessage: string,
    ): Promise<void> {
        const languageClient = this.languageClient;
        if (!languageClient) {
            return;
        }

        // update status while restarting
        statusBarItem.text = "Destack: Restarting";
        statusBarItem.tooltip = "Destack language server is restarting";

        // stop the language client first
        await languageClient.stop();

        // ensure previous server process is terminated
        const languageServerProcess = this.languageServerProcess;
        this.languageServerProcess = undefined;
        await stopServerProcess(languageServerProcess, serverLog);

        // start client again and notify user
        await languageClient.start();
        vscode.window.showInformationMessage(successMessage);
    }

    /**
     * Execute a workspace command through the language server.
     */
    private async executeServerCommand(
        statusBarItem: vscode.StatusBarItem,
        actionLabel: string,
        command: string,
        successMessage: string,
        failurePrefix: string,
    ): Promise<void> {
        const languageClient = this.languageClient;
        if (!languageClient) {
            vscode.window.showErrorMessage("Destack server is not running.");
            return;
        }

        // execute the server command request
        statusBarItem.text = `Destack: ${actionLabel}`;
        try {
            await languageClient.sendRequest("workspace/executeCommand", {
                command,
                arguments: [],
            });

            statusBarItem.text = "Destack: Ready";
            vscode.window.showInformationMessage(successMessage);
        } catch (error: any) {
            const errorMessage = error?.message || error;
            statusBarItem.text = "Destack: Ready";
            vscode.window.showErrorMessage(`${failurePrefix}: ${errorMessage}`);
        }
    }
}
