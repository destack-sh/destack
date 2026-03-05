import assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";

import {
    createWorkspaceProject,
    definitionLocations,
    ensureFixtureReady,
    getDestackTestingApi,
    isRealServerMode,
    openDocument,
    removeDirectory,
    waitForRequestResult,
    withTimeout,
    writeFileTextByUri,
} from "./support";

/** Timeout for workspace command requests in real-server host tests. */
const COMMAND_TIMEOUT_MILLISECONDS = 20_000;

suite("destack real server workspace commands", function () {
    this.timeout(180_000);

    suiteSetup(async function () {
        // skip this suite outside real server mode
        if (!isRealServerMode()) {
            this.skip();
        }

        // wait for extension activation before tests run
        await ensureFixtureReady();
    });

    test("keeps definition requests responsive across workspace commands", async () => {
        // create a workspace local project with one import edge
        const project = createWorkspaceProject("destack-vscode-real-command-", {
            "main.ds": `import { value } from "./lib";
function compute(input: number): number {
    return input + value;
}
const output = compute(1);
`,
            "lib.ds": "export const value = 1;\n",
        });

        try {
            // resolve test API and project URIs
            const api = getDestackTestingApi();
            const mainUri = vscode.Uri.file(path.join(project.path, "main.ds"));
            const libUri = vscode.Uri.file(path.join(project.path, "lib.ds"));
            await openDocument(mainUri);

            // assert baseline definition behavior before command execution
            await assertDefinitionTargetsLibrary(api, mainUri, libUri, {
                line: 2,
                character: 20,
            });

            // execute rescan and verify definition behavior again
            await requestWorkspaceCommand(api, "destack.rescan");
            await assertDefinitionTargetsLibrary(api, mainUri, libUri, {
                line: 2,
                character: 20,
            });

            // apply edits, execute reindex, and verify response behavior
            writeFileTextByUri(libUri, "export const value = 2;\n");
            await requestWorkspaceCommand(api, "destack.reindex");
            await assertDefinitionTargetsLibrary(api, mainUri, libUri, {
                line: 2,
                character: 20,
            });

            // apply another edit, clear cache, and verify response behavior
            writeFileTextByUri(libUri, "export const value = 3;\n");
            await requestWorkspaceCommand(api, "destack.clearCache");
            await assertDefinitionTargetsLibrary(api, mainUri, libUri, {
                line: 2,
                character: 20,
            });
        } finally {
            // clean temporary files after each test run
            removeDirectory(project.path);
        }
    });
});

/** Request one workspace command execution and wait for completion. */
async function requestWorkspaceCommand(
    api: ReturnType<typeof getDestackTestingApi>,
    command: string,
): Promise<void> {
    await withTimeout(
        api.sendRequestForTests("workspace/executeCommand", {
            command,
            arguments: [],
        }),
        COMMAND_TIMEOUT_MILLISECONDS,
        `${command} request timed out`,
    );
}

/** Assert that definition requests resolve to `lib.ds` line 0. */
async function assertDefinitionTargetsLibrary(
    api: ReturnType<typeof getDestackTestingApi>,
    sourceUri: vscode.Uri,
    targetUri: vscode.Uri,
    position: { line: number; character: number },
): Promise<void> {
    // wait for definitions that include the expected target location
    const expectedUri = targetUri.toString();
    const definition = await waitForRequestResult(
        api,
        "textDocument/definition",
        {
            textDocument: { uri: sourceUri.toString() },
            position,
        },
        (result) => {
            if (result == null) {
                return false;
            }

            const locations = definitionLocations(result);
            return locations.some((item) => item.uri == expectedUri && item.startLine == 0);
        },
        `definition request did not resolve to ${expectedUri}`,
    );

    // decode resolved locations for exact assertion
    const locations = definition == null ? [] : definitionLocations(definition);

    // assert exact expected target is present
    assert.ok(
        locations.some((item) => item.uri == expectedUri && item.startLine == 0),
        "expected definition to target lib.ds line 0",
    );
}
