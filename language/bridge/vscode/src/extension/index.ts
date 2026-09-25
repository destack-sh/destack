import type * as vscode from "vscode";

import { DestackExtension } from "./extension";

/** The active extension instance. */
let extension: DestackExtension | undefined;

/** Activate the Destack VS Code extension. */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    const activeExtension = new DestackExtension(context);
    extension = activeExtension;

    await activeExtension.start();
}

/** Deactivate the Destack VS Code extension. */
export async function deactivate(): Promise<void> {
    const activeExtension = extension;
    extension = undefined;

    await activeExtension?.stop();
}
