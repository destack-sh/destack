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
    replaceDocumentText,
    waitForRequestResult,
} from "./support";

suite("destack real server edit flows", function () {
    this.timeout(180_000);

    suiteSetup(async function () {
        // skip this suite outside real server mode
        if (!isRealServerMode()) {
            this.skip();
        }

        // wait for extension activation before tests run
        await ensureFixtureReady();
    });

    test("keeps definition requests responsive across multi-file edits", async () => {
        // create a workspace local project with one import edge
        const project = createWorkspaceProject("destack-vscode-real-flow-", {
            "main.ds": `import { value } from "./lib";
function compute(input: number): number {
    return input + value;
}
const output = compute(1);
`,
            "lib.ds": "export const value = 1;\n",
        });

        try {
            // resolve uris, open project files, and acquire test API
            const api = getDestackTestingApi();
            const mainUri = vscode.Uri.file(path.join(project.path, "main.ds"));
            const libUri = vscode.Uri.file(path.join(project.path, "lib.ds"));
            const mainDocument = await openDocument(mainUri);
            const libDocument = await openDocument(libUri);

            // verify baseline definition target before edits
            await assertDefinitionTargetsLibrary(api, mainUri, libUri, {
                line: 2,
                character: 20,
            });

            // apply edits to both files and verify definition target again
            await replaceDocumentText(libDocument, "export const value = 2;\n");
            await replaceDocumentText(
                mainDocument,
                `import { value } from "./lib";
function render(input: number): number {
    return input + value;
}
const output = render(2);
`,
            );
            await assertDefinitionTargetsLibrary(api, mainUri, libUri, {
                line: 2,
                character: 20,
            });

            // apply another round of edits and verify definition target again
            await replaceDocumentText(
                mainDocument,
                `import { value } from "./lib";
function final_render(input: number): number {
    return input + value;
}
const output = final_render(3);
`,
            );
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
