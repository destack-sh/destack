import type { ChildProcessWithoutNullStreams } from "node:child_process";
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
     * The server log output channel.
     */
    private serverLogOutput: vscode.LogOutputChannel | undefined;

    /**
     * The guard to prevent restart loops during config changes.
     */
    private isConfigurationRestartInFlight = false;

    /**
     * The number of completed restart attempts.
     */
    private restartCount = 0;

    /**
     * The effective server launch settings fingerprint.
     */
    private serverConfigurationFingerprint = "";

    /**
     * Activate the extension runtime and start the language client.
     */
    async activate(context: vscode.ExtensionContext): Promise<void> {
        // create output channels and status ui
        const clientLog = vscode.window.createOutputChannel("Destack Client", { log: true });
        const serverLog = vscode.window.createOutputChannel("Destack Server", { log: true });
        const statusBarItem = this.createStatusBarItem();

        // store extension runtime state
        this.serverLogOutput = serverLog;

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
        this.serverConfigurationFingerprint = this.currentServerConfigurationFingerprint();

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
            this.serverLogOutput = undefined;
            this.isConfigurationRestartInFlight = false;
            this.serverConfigurationFingerprint = "";
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
    private updateStatusBarFromState(state: State, statusBarItem: vscode.StatusBarItem): void {
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
        // register commands and treat duplicate registrations as benign
        const registerCommand = (
            command: string,
            callback: (...args: unknown[]) => unknown,
        ): void => {
            try {
                const disposable = vscode.commands.registerCommand(command, callback);
                context.subscriptions.push(disposable);
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                if (message.includes("already exists")) {
                    clientLog.warn(`skipping duplicate command registration: ${command}`);
                    return;
                }

                throw error;
            }
        };

        // register restart command
        registerCommand("destack.restart", async () => {
            try {
                await this.restartLanguageClient(statusBarItem, serverLog, "Destack restarted.");
            } catch (error: any) {
                const errorMessage = error?.message || error;
                vscode.window.showErrorMessage(`Destack restart failed: ${errorMessage}`);
            }
        });

        // register server operation commands
        registerCommand("destack.rescan", async () => {
            await this.executeServerCommand(
                statusBarItem,
                "Rescanning",
                "destack.rescan",
                "Destack rescan completed.",
                "Destack rescan failed",
            );
        });
        registerCommand("destack.reindex", async () => {
            await this.executeServerCommand(
                statusBarItem,
                "Reindexing",
                "destack.reindex",
                "Destack reindex completed.",
                "Destack reindex failed",
            );
        });
        // register log reveal commands
        registerCommand("destack.showClientLogs", () => {
            clientLog.show(true);
        });
        registerCommand("destack.showServerLogs", () => {
            serverLog.show(true);
        });
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

                // skip restarts when effective launch settings are unchanged
                const nextFingerprint = this.currentServerConfigurationFingerprint();
                if (nextFingerprint == this.serverConfigurationFingerprint) {
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
                    this.serverConfigurationFingerprint = nextFingerprint;
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
            vscode.window.showErrorMessage(`Destack configuration update failed: ${errorMessage}`);
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
        this.restartCount += 1;
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
     * Build a fingerprint for restart relevant server settings.
     */
    private currentServerConfigurationFingerprint(): string {
        const configuration = vscode.workspace.getConfiguration("destack");
        const command = configuration.get<string>("server.command") ?? "";
        const args = configuration.get<string[]>("server.args") ?? [];
        const cwd = configuration.get<string>("server.cwd") ?? "";

        return JSON.stringify({ command, args, cwd });
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
