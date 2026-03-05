import assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";

import {
    createWorkspaceProject,
    ensureFixtureReady,
    getDestackTestingApi,
    isRealServerMode,
    removeDirectory,
    withTimeout,
} from "./tests";

/** Timeout for `workspace/willRenameFiles` requests in LSP tests. */
const RENAME_TIMEOUT_MILLISECONDS = 10_000;

suite("lsp.rename", () => {
    suiteSetup(async function () {
        // skip this suite outside real server mode
        if (!isRealServerMode()) {
            this.skip();
        }

        // wait for extension activation before tests run
        await ensureFixtureReady();
    });

    test("updates imports when a single file is renamed", async () => {
        // create a workspace local project with one import edge
        const project = createWorkspaceProject("destack-vscode-real-rename-", {
            "main.ds": 'import { value } from "./lib";\nconst output = value;\n',
            "lib.ds": "export const value = 1;\n",
        });

        try {
            // prepare rename request uris
            const api = getDestackTestingApi();
            const mainUri = vscode.Uri.file(path.join(project.path, "main.ds"));
            const oldUri = vscode.Uri.file(path.join(project.path, "lib.ds"));
            const newUri = vscode.Uri.file(path.join(project.path, "util.ds"));

            // request rename participation edits and assert request completion
            const workspaceEdit = await requestRenameEdit(api, [
                {
                    oldUri: oldUri.toString(),
                    newUri: newUri.toString(),
                },
            ]);
            assert.notEqual(workspaceEdit, undefined, "expected rename request to complete");

            // assert importing file and rewritten path are included when edits are returned
            if (!workspaceEdit) {
                return;
            }

            const serializedEdit = JSON.stringify(workspaceEdit);
            assert.ok(
                serializedEdit.includes(mainUri.toString()),
                "expected rename participant to edit importing document",
            );
            assert.ok(
                serializedEdit.includes("./util"),
                "expected rename participant to rewrite import path",
            );
        } finally {
            // clean temporary files after each test run
            removeDirectory(project.path);
        }
    });

    test("updates imports when multiple files are renamed", async () => {
        // create a workspace local project with two rename targets
        const project = createWorkspaceProject("destack-vscode-real-rename-multi-", {
            "main.ds": `import { value } from "./lib";
import { item } from "./feature/index";
const output = value + item;
`,
            "lib.ds": "export const value = 1;\n",
            "feature/index.ds": "export const item = 2;\n",
        });

        try {
            // prepare multi-file rename request uris
            const api = getDestackTestingApi();
            const mainUri = vscode.Uri.file(path.join(project.path, "main.ds"));
            const oldLibUri = vscode.Uri.file(path.join(project.path, "lib.ds"));
            const newLibUri = vscode.Uri.file(path.join(project.path, "util.ds"));
            const oldFeatureUri = vscode.Uri.file(path.join(project.path, "feature", "index.ds"));
            const newFeatureUri = vscode.Uri.file(path.join(project.path, "feature", "core.ds"));

            // request rename participation edits and assert request completion
            const workspaceEdit = await requestRenameEdit(api, [
                {
                    oldUri: oldLibUri.toString(),
                    newUri: newLibUri.toString(),
                },
                {
                    oldUri: oldFeatureUri.toString(),
                    newUri: newFeatureUri.toString(),
                },
            ]);
            assert.notEqual(workspaceEdit, undefined, "expected rename request to complete");

            // assert both rewritten import paths are present when edits are returned
            if (!workspaceEdit) {
                return;
            }

            const serializedEdit = JSON.stringify(workspaceEdit);
            assert.ok(
                serializedEdit.includes(mainUri.toString()),
                "expected multi-file rename to edit main document",
            );
            assert.ok(
                serializedEdit.includes("./util"),
                "expected multi-file rename to rewrite ./lib import",
            );
            assert.ok(
                serializedEdit.includes("./feature/core"),
                "expected multi-file rename to rewrite ./feature/index import",
            );
        } finally {
            // clean temporary files after each test run
            removeDirectory(project.path);
        }
    });

    test("handles sequential rename requests in one project", async () => {
        // create a workspace local project with two independent imports
        const project = createWorkspaceProject("destack-vscode-real-rename-seq-", {
            "main.ds": `import { value } from "./lib";
import { helper } from "./helper";
const output = helper(value);
`,
            "lib.ds": "export const value = 1;\n",
            "helper.ds": "export function helper(input: number): number {\n    return input;\n}\n",
        });

        try {
            // resolve API and project URIs for sequential requests
            const api = getDestackTestingApi();
            const mainUri = vscode.Uri.file(path.join(project.path, "main.ds"));
            const oldLibUri = vscode.Uri.file(path.join(project.path, "lib.ds"));
            const newLibUri = vscode.Uri.file(path.join(project.path, "value.ds"));
            const oldHelperUri = vscode.Uri.file(path.join(project.path, "helper.ds"));
            const newHelperUri = vscode.Uri.file(path.join(project.path, "util.ds"));

            // run the first rename request and validate updated lib import
            const firstEdit = await requestRenameEdit(api, [
                {
                    oldUri: oldLibUri.toString(),
                    newUri: newLibUri.toString(),
                },
            ]);
            assert.notEqual(firstEdit, undefined, "expected first rename request to complete");
            if (firstEdit) {
                const firstSerialized = JSON.stringify(firstEdit);
                assert.ok(
                    firstSerialized.includes(mainUri.toString()),
                    "expected first rename to edit main document",
                );
                assert.ok(
                    firstSerialized.includes("./value"),
                    "expected first rename to rewrite ./lib import",
                );
            }

            // run the second rename request and validate updated helper import
            const secondEdit = await requestRenameEdit(api, [
                {
                    oldUri: oldHelperUri.toString(),
                    newUri: newHelperUri.toString(),
                },
            ]);
            assert.notEqual(secondEdit, undefined, "expected second rename request to complete");
            if (secondEdit) {
                const secondSerialized = JSON.stringify(secondEdit);
                assert.ok(
                    secondSerialized.includes(mainUri.toString()),
                    "expected second rename to edit main document",
                );
                assert.ok(
                    secondSerialized.includes("./util"),
                    "expected second rename to rewrite ./helper import",
                );
            }
        } finally {
            // clean temporary files after each test run
            removeDirectory(project.path);
        }
    });
});

/** Request one `workspace/willRenameFiles` edit payload. */
async function requestRenameEdit(
    api: ReturnType<typeof getDestackTestingApi>,
    files: Array<{ oldUri: string; newUri: string }>,
): Promise<unknown> {
    return await withTimeout(
        api.sendRequestForTests<unknown>("workspace/willRenameFiles", {
            files,
        }),
        RENAME_TIMEOUT_MILLISECONDS,
        "willRenameFiles request timed out",
    );
}
