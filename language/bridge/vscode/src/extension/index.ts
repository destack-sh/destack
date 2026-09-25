import type * as vscode from "vscode";

import { TsppExtension } from "./extension";

/** The active extension instance. */
let extension: TsppExtension | undefined;

/** Activate the TS++ VS Code extension. */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    const activeExtension = new TsppExtension(context);
    extension = activeExtension;

    await activeExtension.start();
}

/** Deactivate the TS++ VS Code extension. */
export async function deactivate(): Promise<void> {
    const activeExtension = extension;
    extension = undefined;

    await activeExtension?.stop();
}
