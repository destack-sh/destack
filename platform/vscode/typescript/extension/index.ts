import * as vscode from "vscode";

import { DestackExtensionController } from "./controller";

let controller: DestackExtensionController | undefined;

/// Activate the Destack VSCode extension.
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    const activeController = new DestackExtensionController();
    controller = activeController;
    await activeController.activate(context);
}

/// Deactivate the Destack VSCode extension.
export async function deactivate(): Promise<void> {
    const activeController = controller;
    controller = undefined;

    await activeController?.deactivate();
}
