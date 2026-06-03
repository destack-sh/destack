import type * as vscode from "vscode";

import { DestackExtensionController } from "./controller";

/** The singleton controller instance for this extension host. */
let controller: DestackExtensionController | undefined;

/** The in flight activation promise for concurrent activation calls. */
let activation: Promise<void> | undefined;

/** Activate the Destack VSCode extension. */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    // await existing activation when startup is already in progress
    if (activation) {
        await activation;
        return;
    }

    // make extension activation idempotent across overlapping activation events
    activation = (async () => {
        try {
            // create or reuse the controller instance
            const activeController = controller ?? new DestackExtensionController();
            controller = activeController;
            await activeController.activate(context);
        } catch (error) {
            // clear activation state when startup fails
            activation = undefined;
            throw error;
        }
    })();

    return await activation;
}

/** Deactivate the Destack VSCode extension. */
export async function deactivate(): Promise<void> {
    // capture and clear extension state first
    const activeController = controller;
    controller = undefined;
    activation = undefined;

    // stop controller runtime when active
    await activeController?.deactivate();
}
