import assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";

import type { DestackExtensionApi } from "../../extension/index";

/** Timeout used for raw LSP request helpers in host tests. */
const REQUEST_TIMEOUT_MILLISECONDS = 30_000;

/** Timeout used while waiting for extension runtime readiness. */
const RUNTIME_READY_TIMEOUT_MILLISECONDS = 30_000;

/** Retry count used for LSP request convergence checks in host tests. */
const REQUEST_RETRY_ATTEMPTS = 40;

/** Commands that the extension must register after activation. */
export const DESTACK_COMMANDS = [
    "destack.restart",
    "destack.rescan",
    "destack.reindex",
    "destack.clearCache",
];

/** The normalized definition location shape used by assertions. */
export type DefinitionLocation = {
    /** The destination document URI string. */
    uri: string;
    /** The destination start line. */
    startLine: number;
};

/** Return true when tests are running against the real Destack server. */
export function isRealServerMode(): boolean {
    return process.env.DESTACK_VSCODE_HOST_REAL_SERVER == "1";
}

/** Return the installed Destack extension instance. */
export function findDestackExtension(): vscode.Extension<unknown> | undefined {
    // resolve by published extension id first
    const extensionById = vscode.extensions.getExtension("symbol-industries.destack");
    if (extensionById) {
        return extensionById;
    }

    // fall back to package name lookup
    return vscode.extensions.all.find((extension) => extension.packageJSON?.name == "destack");
}

/** Wait for extension activation and command registration. */
export async function waitForExtensionActivation(
    extension: vscode.Extension<unknown>,
): Promise<void> {
    // retry until extension is active and commands are registered
    const attempts = 40;

    for (let attempt = 0; attempt < attempts; attempt += 1) {
        // validate active extension state and command registration
        if (extension.isActive) {
            const commands = await vscode.commands.getCommands(true);
            if (commands.includes("destack.restart")) {
                return;
            }
        }

        // wait before the next activation poll
        await delay(200);
    }

    assert.fail("destack extension did not activate");
}

/** Ensure extension activation is complete and runtime is ready. */
export async function ensureFixtureReady(): Promise<void> {
    // resolve extension from registry
    const extension = findDestackExtension();
    assert.ok(extension, "destack extension should be present");

    // activate extension and wait for command registration
    await extension.activate();
    await waitForExtensionActivation(extension);

    // wait until the language client is running and not restarting
    const api = getDestackTestingApi();
    await waitForRuntimeReady(api);
}

/** Return the active workspace root path. */
export function workspaceRootPath(): string {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    assert.ok(workspaceFolder, "workspace folder should be available");

    return workspaceFolder.uri.fsPath;
}

/** Return the URI for the default fixture document. */
export function fixtureDocumentUri(): vscode.Uri {
    const documentPath = path.join(workspaceRootPath(), "main.ds");
    return vscode.Uri.file(documentPath);
}

/** Write full text content to a file path. */
export function writeFileText(filePath: string, text: string): void {
    fs.writeFileSync(filePath, text, "utf8");
}

/** Write full text content to a file URI. */
export function writeFileTextByUri(uri: vscode.Uri, text: string): void {
    writeFileText(uri.fsPath, text);
}

/** Open and show a document by URI. */
export async function openDocument(uri: vscode.Uri): Promise<vscode.TextDocument> {
    const document = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(document);

    return document;
}

/** Return a range spanning the full document text. */
export function fullDocumentRange(document: vscode.TextDocument): vscode.Range {
    const start = document.positionAt(0);
    const end = document.positionAt(document.getText().length);

    return new vscode.Range(start, end);
}

/** Replace the full document with the provided text. */
export async function replaceDocumentText(
    document: vscode.TextDocument,
    text: string,
): Promise<void> {
    // apply a full-buffer edit through VSCode to trigger LSP change events
    const editor = await vscode.window.showTextDocument(document);
    const didApply = await editor.edit((builder) => {
        builder.replace(fullDocumentRange(document), text);
    });
    assert.equal(didApply, true, "document edit should be applied");

    // force the save so on-disk state and editor state stay aligned
    const didSave = await document.save();
    assert.equal(didSave, true, "document save should succeed");
}

/** Return the extension testing API for raw request and notification calls. */
export function getDestackTestingApi(): DestackExtensionApi {
    // resolve extension from registry
    const extension = findDestackExtension();
    assert.ok(extension, "destack extension should be present");

    // validate exported testing methods
    const api = extension.exports as Partial<DestackExtensionApi>;
    assert.equal(typeof api.sendRequestForTests, "function");
    assert.equal(typeof api.sendNotificationForTests, "function");
    assert.equal(typeof api.getRuntimeStateForTests, "function");

    return api as DestackExtensionApi;
}

/** Wait for diagnostics to satisfy a predicate and return the observed items. */
export async function waitForDiagnosticItems(
    api: DestackExtensionApi,
    uri: vscode.Uri,
    predicate: (items: unknown[]) => boolean,
    failureMessage: string,
): Promise<unknown[]> {
    // retry pull diagnostics until predicate passes
    const attempts = 40;
    let lastItems: unknown[] = [];

    for (let attempt = 0; attempt < attempts; attempt += 1) {
        // request diagnostics for the target document
        const report = await withTimeout(
            api.sendRequestForTests<unknown>("textDocument/diagnostic", {
                textDocument: { uri: uri.toString() },
                identifier: null,
                previousResultId: null,
                workDoneProgressParams: { workDoneToken: null },
                partialResultParams: { partialResultToken: null },
            }),
            REQUEST_TIMEOUT_MILLISECONDS,
            "textDocument/diagnostic request timed out",
        );

        // parse items from the report payload
        const diagnosticItems = readDiagnosticItems(report);

        // return as soon as the predicate matches
        if (predicate(diagnosticItems)) {
            return diagnosticItems;
        }

        // store latest result and wait before retrying
        lastItems = diagnosticItems;
        await delay(150);
    }

    assert.fail(`${failureMessage}; last diagnostic count=${lastItems.length}`);
}

/** Create a temporary project under the active workspace root. */
export function createWorkspaceProject(
    prefix: string,
    files: Record<string, string>,
): { path: string; uri: vscode.Uri } {
    // allocate temporary project root inside the active workspace
    const projectPath = fs.mkdtempSync(path.join(workspaceRootPath(), prefix));

    // write each project file to disk
    for (const [relativePath, content] of Object.entries(files)) {
        const filePath = path.join(projectPath, relativePath);
        fs.mkdirSync(path.dirname(filePath), { recursive: true });
        fs.writeFileSync(filePath, content, "utf8");
    }

    return {
        path: projectPath,
        uri: vscode.Uri.file(projectPath),
    };
}

/** Generate many source files to create heavier workspace diagnostics traffic. */
export function seedDiagnosticLoad(workspaceRoot: string, fileCount: number): void {
    // create the generated directory if needed
    const generatedDirectory = path.join(workspaceRoot, "generated");
    fs.mkdirSync(generatedDirectory, { recursive: true });

    // generate many small files for workspace diagnostic load
    for (let index = 0; index < fileCount; index += 1) {
        const fileName = `diag_${index.toString().padStart(4, "0")}.ds`;
        const filePath = path.join(generatedDirectory, fileName);
        const text = `export function item_${index}(value: number): number {
    const value_copy = value;
    return value_copy;
}
`;
        fs.writeFileSync(filePath, text, "utf8");
    }
}

/** Parse definition results into a normalized location list. */
export function definitionLocations(result: unknown): DefinitionLocation[] {
    // map array payloads directly
    if (Array.isArray(result)) {
        return result.flatMap((item) => normalizeDefinitionLocation(item));
    }

    // map one location payload when present
    return normalizeDefinitionLocation(result);
}

/** Sleep for the provided duration. */
export async function delay(milliseconds: number): Promise<void> {
    await new Promise<void>((resolve) => {
        setTimeout(resolve, milliseconds);
    });
}

/** Resolve or reject a promise within the provided timeout. */
export async function withTimeout<T>(
    promise: Promise<T>,
    timeoutMilliseconds: number,
    message: string,
): Promise<T> {
    // build a timeout promise for the request
    const timeout = new Promise<never>((_, reject) => {
        setTimeout(() => {
            reject(new Error(message));
        }, timeoutMilliseconds);
    });

    return await Promise.race([promise, timeout]);
}

/** Retry an LSP request until the response matches the predicate. */
export async function waitForRequestResult<T>(
    api: DestackExtensionApi,
    method: string,
    params: unknown,
    predicate: (result: T) => boolean,
    failureMessage: string,
): Promise<T> {
    // retry requests to absorb startup jitter in real server mode
    const attempts = REQUEST_RETRY_ATTEMPTS;
    let lastError: unknown;

    for (let attempt = 0; attempt < attempts; attempt += 1) {
        // skip request attempts while the client is restarting
        const runtimeState = api.getRuntimeStateForTests();
        if (runtimeState.clientState != "running" || runtimeState.isConfigurationRestartInFlight) {
            await delay(200);
            continue;
        }

        try {
            // send request and enforce timeout
            const result = await withTimeout(
                api.sendRequestForTests<T>(method, params),
                REQUEST_TIMEOUT_MILLISECONDS,
                `${method} request timed out`,
            );

            // return once predicate passes
            if (predicate(result)) {
                return result;
            }
        } catch (error) {
            // keep the last request error for diagnostics
            lastError = error;
        }

        // wait before retrying
        await delay(200);
    }

    // include the last error when available
    if (lastError) {
        const message = lastError instanceof Error ? lastError.message : String(lastError);
        const runtimeState = api.getRuntimeStateForTests();
        assert.fail(
            `${failureMessage}; last error: ${message}; runtime=${JSON.stringify(runtimeState)}`,
        );
    }

    const runtimeState = api.getRuntimeStateForTests();
    assert.fail(`${failureMessage}; runtime=${JSON.stringify(runtimeState)}`);
}

/** Wait for definition-provider locations to satisfy a predicate. */
export async function waitForDefinitionLocations(
    api: DestackExtensionApi,
    uri: vscode.Uri,
    position: { line: number; character: number },
    predicate: (locations: DefinitionLocation[]) => boolean,
    failureMessage: string,
): Promise<DefinitionLocation[]> {
    // retry provider requests to absorb analysis and synchronization jitter
    let lastLocations: DefinitionLocation[] = [];
    let lastError: unknown;
    const editorPosition = new vscode.Position(position.line, position.character);

    for (let attempt = 0; attempt < REQUEST_RETRY_ATTEMPTS; attempt += 1) {
        // skip provider requests while the language client is restarting
        const runtimeState = api.getRuntimeStateForTests();
        if (runtimeState.clientState != "running" || runtimeState.isConfigurationRestartInFlight) {
            await delay(200);
            continue;
        }

        try {
            // request definitions through the standard vscode provider command
            const result = await withTimeout(
                Promise.resolve(
                    vscode.commands.executeCommand<unknown>(
                        "vscode.executeDefinitionProvider",
                        uri,
                        editorPosition,
                    ),
                ),
                REQUEST_TIMEOUT_MILLISECONDS,
                "vscode.executeDefinitionProvider request timed out",
            );

            // normalize and evaluate the returned locations
            const locations = definitionLocations(result);
            if (predicate(locations)) {
                return locations;
            }

            // preserve last payload for failure diagnostics
            lastLocations = locations;
        } catch (error) {
            // preserve the last provider failure and continue retrying
            lastError = error;
        }

        await delay(200);
    }

    // include the last provider error when available
    if (lastError) {
        const message = lastError instanceof Error ? lastError.message : String(lastError);
        const runtimeState = api.getRuntimeStateForTests();
        assert.fail(
            `${failureMessage}; last error: ${message}; runtime=${JSON.stringify(runtimeState)}`,
        );
    }

    assert.fail(`${failureMessage}; last location count=${lastLocations.length}`);
}

/** Remove a directory recursively and ignore missing paths. */
export function removeDirectory(directoryPath: string): void {
    fs.rmSync(directoryPath, { recursive: true, force: true });
}

/** Read diagnostic items from a textDocument/diagnostic result payload. */
function readDiagnosticItems(report: unknown): unknown[] {
    // reject non object payloads
    if (!report || typeof report != "object") {
        return [];
    }

    // extract the items field
    const items = (report as { items?: unknown }).items;
    if (!Array.isArray(items)) {
        return [];
    }

    return items;
}

/** Normalize one definition payload into location entries. */
function normalizeDefinitionLocation(result: unknown): DefinitionLocation[] {
    // reject non-object payloads
    if (!result || typeof result != "object") {
        return [];
    }

    // normalize location payloads
    const locationUri = normalizeLocationUri((result as { uri?: unknown }).uri);
    const locationRange = (result as { range?: unknown }).range;
    if (locationUri && isRangeLike(locationRange)) {
        return [
            {
                uri: locationUri,
                startLine: locationRange.start.line,
            },
        ];
    }

    // normalize location-link payloads
    const linkTargetUri = normalizeLocationUri((result as { targetUri?: unknown }).targetUri);
    const linkTargetRange = (result as { targetRange?: unknown }).targetRange;
    if (linkTargetUri && isRangeLike(linkTargetRange)) {
        return [
            {
                uri: linkTargetUri,
                startLine: linkTargetRange.start.line,
            },
        ];
    }

    return [];
}

/** Normalize uri-like values from LSP and VSCode provider payloads. */
function normalizeLocationUri(value: unknown): string | undefined {
    // pass through string uri values directly
    if (typeof value == "string") {
        return value;
    }

    // normalize vscode.Uri and other uri-like objects through toString
    if (
        value &&
        typeof value == "object" &&
        typeof (value as { toString?: unknown }).toString == "function"
    ) {
        const uri = String((value as { toString: () => string }).toString());
        if (uri.length > 0) {
            return uri;
        }
    }

    return undefined;
}

/** Return true when a value matches an LSP range shape. */
function isRangeLike(value: unknown): value is { start: { line: number } } {
    // reject non-object range values
    if (!value || typeof value != "object") {
        return false;
    }

    // extract start position for line validation
    const start = (value as { start?: unknown }).start;
    if (!start || typeof start != "object") {
        return false;
    }

    return typeof (start as { line?: unknown }).line == "number";
}

/** Wait until the language client is fully running for host test requests. */
async function waitForRuntimeReady(api: DestackExtensionApi): Promise<void> {
    // retry until runtime reports a stable running state
    const startedAt = Date.now();
    while (Date.now() - startedAt < RUNTIME_READY_TIMEOUT_MILLISECONDS) {
        const runtimeState = api.getRuntimeStateForTests();
        if (runtimeState.clientState == "running" && !runtimeState.isConfigurationRestartInFlight) {
            return;
        }

        await delay(100);
    }

    const runtimeState = api.getRuntimeStateForTests();
    assert.fail(`extension runtime did not become ready: ${JSON.stringify(runtimeState)}`);
}
