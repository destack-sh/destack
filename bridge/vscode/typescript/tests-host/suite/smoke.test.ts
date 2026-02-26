import assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";

const DESTACK_COMMANDS = [
    "destack.restart",
    "destack.rescan",
    "destack.reindex",
    "destack.clearCache",
];

suite("destack extension host", () => {
    suiteSetup(async () => {
        const extension = findDestackExtension();
        assert.ok(extension, "destack extension should be present");
        await openFixtureDocument();
        await waitForExtensionActivation(extension);
    });

    test("activates and registers commands", async () => {
        const extension = findDestackExtension();
        assert.ok(extension, "destack extension should be present");
        assert.equal(extension.isActive, true, "destack extension should be active");

        const commands = await vscode.commands.getCommands(true);
        for (const command of DESTACK_COMMANDS) {
            assert.ok(commands.includes(command), `missing command: ${command}`);
        }
    });

    test("serves definition requests", async () => {
        const extension = findDestackExtension();
        assert.ok(extension, "destack extension should be present");
        assert.equal(extension.isActive, true, "destack extension should be active");

        const document = await openFixtureDocument();

        const position = new vscode.Position(1, 10);
        const locations = await waitForDefinitions(document.uri, position);
        assert.ok(Array.isArray(locations), "definition provider should return an array");
        assert.ok(locations.length > 0, "definition result should not be empty");
    });
});

function findDestackExtension(): vscode.Extension<unknown> | undefined {
    const byId = vscode.extensions.getExtension("symbol-industries.destack");
    if (byId) {
        return byId;
    }

    return vscode.extensions.all.find((extension) => extension.packageJSON?.name == "destack");
}

function workspaceRootPath(): string {
    const workspaceFolders = vscode.workspace.workspaceFolders ?? [];
    const workspaceFolder = workspaceFolders[0];
    assert.ok(workspaceFolder, "workspace folder should be available");

    return workspaceFolder.uri.fsPath;
}

async function waitForDefinitions(
    uri: vscode.Uri,
    position: vscode.Position,
): Promise<vscode.Location[] | vscode.LocationLink[]> {
    const attempts = 20;

    for (let attempt = 0; attempt < attempts; attempt += 1) {
        const locations =
            await vscode.commands.executeCommand<vscode.Location[] | vscode.LocationLink[]>(
                "vscode.executeDefinitionProvider",
                uri,
                position,
            );
        if (Array.isArray(locations) && locations.length > 0) {
            return locations;
        }

        await delay(250);
    }

    return [];
}

async function delay(milliseconds: number): Promise<void> {
    await new Promise<void>((resolve) => {
        setTimeout(resolve, milliseconds);
    });
}

async function waitForExtensionActivation(
    extension: vscode.Extension<unknown>,
): Promise<void> {
    const attempts = 30;

    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (extension.isActive) {
            return;
        }

        await delay(200);
    }

    assert.fail("destack extension did not activate");
}

async function openFixtureDocument(): Promise<vscode.TextDocument> {
    const workspaceRoot = workspaceRootPath();
    const documentPath = path.join(workspaceRoot, "main.ds");
    const document = await vscode.workspace.openTextDocument(documentPath);
    await vscode.window.showTextDocument(document);

    return document;
}
