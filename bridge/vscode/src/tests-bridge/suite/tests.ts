import assert from "node:assert/strict";
import * as path from "node:path";

import * as vscode from "vscode";

import type { DestackExtensionApi } from "../../extension/index";

/** Timeout used while waiting for extension runtime readiness. */
const RUNTIME_READY_TIMEOUT_MILLISECONDS = 30_000;

/** Timeout used for definition provider command calls. */
const REQUEST_TIMEOUT_MILLISECONDS = 15_000;

/** Retry count used for definition provider convergence checks. */
const DEFINITION_REQUEST_RETRY_ATTEMPTS = 40;

/** Commands that the extension must register after activation. */
export const DESTACK_COMMANDS = [
    "destack.restart",
    "destack.rescan",
    "destack.reindex",
    "destack.clearCache",
    "destack.showClientLogs",
    "destack.showServerLogs",
];

/** One normalized definition location used by smoke assertions. */
export type DefinitionLocation = {
    /** The destination document URI string. */
    uri: string;
    /** The destination start line. */
    startLine: number;
    /** The destination start character. */
    startCharacter: number;
    /** The destination end line. */
    endLine: number;
    /** The destination end character. */
    endCharacter: number;
};

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

/** Return the extension testing API for raw request and runtime checks. */
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

/** Ensure extension activation is complete and runtime is ready. */
export async function ensureBridgeReady(): Promise<void> {
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

/** Return the URI for the default fixture document. */
export function fixtureDocumentUri(): vscode.Uri {
    // resolve the current workspace root path first
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    assert.ok(workspaceFolder, "workspace folder should be available");

    // resolve the default fixture document path
    const documentPath = path.join(workspaceFolder.uri.fsPath, "main.ds");

    return vscode.Uri.file(documentPath);
}

/** Open and show a document by URI. */
export async function openDocument(uri: vscode.Uri): Promise<vscode.TextDocument> {
    // load the document through vscode workspace apis
    const document = await vscode.workspace.openTextDocument(uri);

    // show the document to ensure provider requests use an active editor context
    await vscode.window.showTextDocument(document);

    return document;
}

/** Build one exact definition location expectation. */
export function definitionLocation(
    uri: vscode.Uri | string,
    startLine: number,
    startCharacter: number,
    endLine: number,
    endCharacter: number,
): DefinitionLocation {
    // normalize uri values to the string representation used by provider payloads
    const normalizedUri = typeof uri == "string" ? uri : uri.toString();

    return {
        uri: normalizedUri,
        startLine,
        startCharacter,
        endLine,
        endCharacter,
    };
}

/** Assert that the definition provider returns one exact expected location. */
export async function assertDefinitionLocation(
    api: DestackExtensionApi,
    sourceUri: vscode.Uri,
    position: { line: number; character: number },
    expectedLocation: DefinitionLocation,
    failureMessage: string,
): Promise<void> {
    // wait until the exact target location is present in provider results
    const locations = await waitForDefinitionLocations(
        api,
        sourceUri,
        position,
        (items) => items.some((item) => isExactDefinitionLocation(item, expectedLocation)),
        failureMessage,
    );

    // assert the exact target location is present
    assert.ok(
        locations.some((item) => isExactDefinitionLocation(item, expectedLocation)),
        "expected definition to include the exact target location",
    );
}

/** Sleep for the provided duration. */
export async function delay(milliseconds: number): Promise<void> {
    // resolve after one timeout tick
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

/** Wait for extension activation and command registration. */
async function waitForExtensionActivation(
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

/** Wait for definition-provider locations to satisfy a predicate. */
async function waitForDefinitionLocations(
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

    for (let attempt = 0; attempt < DEFINITION_REQUEST_RETRY_ATTEMPTS; attempt += 1) {
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

    assert.fail(
        `${failureMessage}; last location count=${lastLocations.length}; last locations=${JSON.stringify(lastLocations)}`,
    );
}

/** Parse definition results into a normalized location list. */
function definitionLocations(result: unknown): DefinitionLocation[] {
    // map array payloads directly
    if (Array.isArray(result)) {
        return result.flatMap((item) => normalizeDefinitionLocation(item));
    }

    // map one location payload when present
    return normalizeDefinitionLocation(result);
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
                startCharacter: locationRange.start.character,
                endLine: locationRange.end.line,
                endCharacter: locationRange.end.character,
            },
        ];
    }

    // normalize location-link payloads
    const linkTargetUri = normalizeLocationUri((result as { targetUri?: unknown }).targetUri);
    const linkTargetSelectionRange = (result as { targetSelectionRange?: unknown })
        .targetSelectionRange;
    const linkTargetRange = (result as { targetRange?: unknown }).targetRange;
    const normalizedLinkRange = isRangeLike(linkTargetSelectionRange)
        ? linkTargetSelectionRange
        : linkTargetRange;
    if (linkTargetUri && isRangeLike(normalizedLinkRange)) {
        return [
            {
                uri: linkTargetUri,
                startLine: normalizedLinkRange.start.line,
                startCharacter: normalizedLinkRange.start.character,
                endLine: normalizedLinkRange.end.line,
                endCharacter: normalizedLinkRange.end.character,
            },
        ];
    }

    return [];
}

/** Normalize uri-like values from LSP and vscode provider payloads. */
function normalizeLocationUri(value: unknown): string | undefined {
    // pass through string uri values directly
    if (typeof value == "string") {
        return value;
    }

    // normalize vscode uri and other uri-like objects through toString
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
function isRangeLike(
    value: unknown,
): value is { start: { line: number; character: number }; end: { line: number; character: number } } {
    // reject non-object range values
    if (!value || typeof value != "object") {
        return false;
    }

    // extract start and end positions for shape validation
    const start = (value as { start?: unknown }).start;
    const end = (value as { end?: unknown }).end;
    if (!start || typeof start != "object" || !end || typeof end != "object") {
        return false;
    }

    return (
        typeof (start as { line?: unknown }).line == "number" &&
        typeof (start as { character?: unknown }).character == "number" &&
        typeof (end as { line?: unknown }).line == "number" &&
        typeof (end as { character?: unknown }).character == "number"
    );
}

/** Return true when two normalized definition locations are exactly equal. */
function isExactDefinitionLocation(actual: DefinitionLocation, expected: DefinitionLocation): boolean {
    return (
        actual.uri == expected.uri &&
        actual.startLine == expected.startLine &&
        actual.startCharacter == expected.startCharacter &&
        actual.endLine == expected.endLine &&
        actual.endCharacter == expected.endCharacter
    );
}

/** Wait until the language client is fully running for bridge test requests. */
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
