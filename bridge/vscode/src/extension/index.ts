import type * as vscode from "vscode";

import { DestackExtensionController } from "./controller";
import type { DestackRuntimeState } from "./types";

/** The singleton controller instance for this extension host. */
let controller: DestackExtensionController | undefined;

/** The exported API cache for repeated activation calls. */
let api: DestackExtensionApi | undefined;

/** The in flight activation promise for concurrent activation calls. */
let activation: Promise<DestackExtensionApi> | undefined;

/** Public extension API for integration tests. */
export type DestackExtensionApi = {
    /** Send a raw LSP request through the active language client. */
    sendRequestForTests<T = unknown>(method: string, params: unknown): Promise<T>;

    /** Send a raw LSP notification through the active language client. */
    sendNotificationForTests(method: string, params: unknown): Promise<void>;

    /** Return extension runtime state for host test synchronization. */
    getRuntimeStateForTests(): DestackRuntimeState;
};

/** Activate the Destack VSCode extension. */
export async function activate(context: vscode.ExtensionContext): Promise<DestackExtensionApi> {
    // return cached api once activation has completed
    if (api) {
        return api;
    }

    // await existing activation when startup is already in progress
    if (activation) {
        return activation;
    }

    // make extension activation idempotent across overlapping activation events
    activation = (async () => {
        try {
            // create or reuse the controller instance
            const activeController = controller ?? new DestackExtensionController();
            controller = activeController;
            await activeController.activate(context);

            // store the stable exported api proxy
            api = {
                sendRequestForTests: (method, params) =>
                    activeController.sendRequestForTests(method, params),
                sendNotificationForTests: (method, params) =>
                    activeController.sendNotificationForTests(method, params),
                getRuntimeStateForTests: () => activeController.getRuntimeStateForTests(),
            };
            return api;
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
    api = undefined;
    activation = undefined;

    // stop controller runtime when active
    await activeController?.deactivate();
}
