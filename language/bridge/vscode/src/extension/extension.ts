import * as vscode from "vscode";
import { LanguageClient, type LanguageClientOptions } from "vscode-languageclient/node";

import { ServerCommand } from "./server";

/** One source position carried by a Destack CodeLens command. */
type SourcePosition = {
    /** The zero-based source line. */
    line: number;
    /** The zero-based UTF-16 source character. */
    character: number;
};

/** The active Destack VS Code extension. */
export class DestackExtension {
    /** Extension context. */
    private readonly context: vscode.ExtensionContext;

    /** The active language client. */
    private client: LanguageClient | undefined;
    /** The last queued language client change. */
    private pendingClientChange = Promise.resolve();
    /** Whether this extension instance has stopped. */
    private isStopped = false;
    /** The shared extension log. */
    private readonly log: vscode.LogOutputChannel;

    /** Create one extension owned by its VS Code context. */
    constructor(context: vscode.ExtensionContext) {
        this.context = context;

        this.log = vscode.window.createOutputChannel("Destack", { log: true });
        context.subscriptions.push(this.log);
    }

    /** Start the extension and its language client. */
    async start(): Promise<void> {
        this.registerCommands();
        this.registerRestart();
        await this.report("start Destack", () => this.restart());
    }

    /** Stop the active language client. */
    async stop(): Promise<void> {
        this.isStopped = true;

        await this.enqueueClientChange(() => this.stopClient());
    }

    /** Stop the active language client without changing extension state. */
    private async stopClient(): Promise<void> {
        const client = this.client;
        this.client = undefined;
        if (client) {
            await client.stop();
        }
    }

    /** Start a newly resolved language client. */
    private async startClient(): Promise<void> {
        const command = await ServerCommand.resolve(this.context);
        this.log.info(`event=server.start executable=${JSON.stringify(command.command)}`);

        const client = new LanguageClient(
            "destack",
            "Destack",
            command.executable(),
            this.clientOptions(),
        );
        await client.start();
        this.client = client;
    }

    /** Build the complete language client configuration. */
    private clientOptions(): LanguageClientOptions {
        return {
            documentSelector: [
                { language: "destack-ds", scheme: "file" },
                { language: "destack-ds", scheme: "destack" },
            ],
            outputChannel: this.log,
            traceOutputChannel: this.log,
            initializationOptions: {
                codeLensCommands: ["references", "implementations"],
            },
            synchronize: {
                configurationSection: ["destack.completion", "destack.inlayHints"],
            },
        };
    }

    /** Restart the language client with current command settings. */
    private async restart(): Promise<void> {
        await this.enqueueClientChange(async () => {
            await this.stopClient();
            if (!this.isStopped) {
                await this.startClient();
            }
        });
    }

    /** Enqueue one ordered language client change. */
    private enqueueClientChange(change: () => Promise<void>): Promise<void> {
        const pendingClientChange = this.pendingClientChange.then(change, change);
        this.pendingClientChange = pendingClientChange;

        return pendingClientChange;
    }

    /** Register commands implemented by the extension. */
    private registerCommands(): void {
        this.context.subscriptions.push(
            vscode.commands.registerCommand("destack.restart", async () => {
                await this.report("restart Destack", () => this.restart());
            }),
            vscode.commands.registerCommand("destack.showLogs", () => {
                this.log.show(true);
            }),
            vscode.commands.registerCommand(
                "destack.showReferences",
                async (uri: string, position: SourcePosition) => {
                    await this.report("show Destack references", async () =>
                        this.showLocations(
                            uri,
                            position,
                            "vscode.executeReferenceProvider",
                            "No references found.",
                        ),
                    );
                },
            ),
            vscode.commands.registerCommand(
                "destack.showImplementations",
                async (uri: string, position: SourcePosition) => {
                    await this.report("show Destack implementations", async () =>
                        this.showLocations(
                            uri,
                            position,
                            "vscode.executeImplementationProvider",
                            "No implementations found.",
                        ),
                    );
                },
            ),
        );
    }

    /** Restart when language server command selection changes. */
    private registerRestart(): void {
        this.context.subscriptions.push(
            vscode.workspace.onDidChangeConfiguration(async (event) => {
                if (!event.affectsConfiguration("destack.server")) {
                    return;
                }

                await this.report("apply Destack server settings", () => this.restart());
            }),
            vscode.workspace.onDidGrantWorkspaceTrust(async () => {
                await this.report("apply Destack workspace trust", () => this.restart());
            }),
        );
    }

    /** Show provider locations requested by one CodeLens. */
    private async showLocations(
        uriValue: string,
        positionValue: SourcePosition,
        provider: "vscode.executeReferenceProvider" | "vscode.executeImplementationProvider",
        emptyMessage: string,
    ): Promise<void> {
        if (
            typeof uriValue !== "string" ||
            !Number.isInteger(positionValue?.line) ||
            !Number.isInteger(positionValue?.character)
        ) {
            throw new TypeError("Destack CodeLens has invalid source coordinates");
        }

        const uri = vscode.Uri.parse(uriValue);
        const position = new vscode.Position(positionValue.line, positionValue.character);
        const results =
            (await vscode.commands.executeCommand<Array<vscode.Location | vscode.LocationLink>>(
                provider,
                uri,
                position,
            )) ?? [];
        const locations = results.map((result) =>
            "targetUri" in result
                ? new vscode.Location(
                      result.targetUri,
                      result.targetSelectionRange ?? result.targetRange,
                  )
                : result,
        );

        await vscode.commands.executeCommand(
            "editor.action.goToLocations",
            uri,
            position,
            locations,
            "peek",
            emptyMessage,
        );
    }

    /** Report one command failure through the extension log and editor UI. */
    private async report(action: string, run: () => Promise<void>): Promise<void> {
        try {
            await run();
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            this.log.error(
                `event=extension.operation.failed operation=${JSON.stringify(action)} error=${JSON.stringify(message)}`,
            );
            const selection = await vscode.window.showErrorMessage(
                `Failed to ${action}: ${message}`,
                "Show Logs",
            );
            if (selection === "Show Logs") {
                this.log.show(true);
            }
        }
    }
}
