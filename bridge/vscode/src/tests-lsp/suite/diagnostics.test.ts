import assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";

import {
    createWorkspaceProject,
    ensureFixtureReady,
    fixtureDocumentUri,
    getDestackTestingApi,
    isRealServerMode,
    removeDirectory,
    waitForDiagnosticItems,
    writeFileTextByUri,
} from "./tests";

/** Baseline library source for multi-file diagnostic flows. */
const BASELINE_LIB_TEXT = "export const value = 1;\n";

/** Parse-invalid library source used to force a state transition. */
const INVALID_LIB_TEXT = "export const value = ;\n";

/** Updated parse-valid library source after the invalid transition. */
const UPDATED_LIB_TEXT = "export const value = 2;\n";

/** Baseline helper source for multi-file diagnostic flows. */
const BASELINE_HELPER_TEXT = `export function helper(input: number): number {
    const output = input;
    return output;
}
`;

/** Updated helper source used to trigger an additional transition. */
const UPDATED_HELPER_TEXT = `export function helper(input: number): number {
    const output = input + 1;
    return output;
}
`;

suite("lsp.diagnostics", () => {
    suiteSetup(async function () {
        // skip this suite outside real server mode
        if (!isRealServerMode()) {
            this.skip();
        }

        // wait for extension activation before tests run
        await ensureFixtureReady();
    });

    test("returns document diagnostics for the fixture file", async () => {
        // resolve fixture uri and acquire test api
        const fixtureUri = fixtureDocumentUri();
        const api = getDestackTestingApi();

        // request diagnostics and assert we get a report payload
        const diagnosticItems = await waitForDiagnosticItems(
            api,
            fixtureUri,
            () => true,
            "expected diagnostic report for fixture file",
        );
        assert.ok(Array.isArray(diagnosticItems), "expected diagnostics array");
    });

    test("tracks diagnostics through multi-file disk edits", async () => {
        // create a workspace local project with multiple files
        const project = createWorkspaceProject("destack-vscode-real-diag-edit-", {
            "main.ds": `import { value } from "./lib";
import { helper } from "./helper";
const output = helper(value);
`,
            "lib.ds": BASELINE_LIB_TEXT,
            "helper.ds": BASELINE_HELPER_TEXT,
        });

        try {
            // resolve uris and API
            const api = getDestackTestingApi();
            const libUri = vscode.Uri.file(path.join(project.path, "lib.ds"));
            const helperUri = vscode.Uri.file(path.join(project.path, "helper.ds"));

            // sample baseline diagnostics
            const baselineLibDiagnostics = await waitForDiagnosticItems(
                api,
                libUri,
                () => true,
                "expected baseline diagnostics report for lib.ds",
            );
            const baselineHelperDiagnostics = await waitForDiagnosticItems(
                api,
                helperUri,
                () => true,
                "expected baseline diagnostics report for helper.ds",
            );

            // apply invalid then valid edits to lib.ds and sample diagnostics
            writeFileTextByUri(libUri, INVALID_LIB_TEXT);
            const invalidLibDiagnostics = await waitForDiagnosticItems(
                api,
                libUri,
                () => true,
                "expected diagnostics report for lib.ds after invalid edit",
            );
            writeFileTextByUri(libUri, UPDATED_LIB_TEXT);
            const restoredLibDiagnostics = await waitForDiagnosticItems(
                api,
                libUri,
                () => true,
                "expected diagnostics report for lib.ds after restore",
            );

            // apply helper edit and sample diagnostics again
            writeFileTextByUri(helperUri, UPDATED_HELPER_TEXT);
            const updatedHelperDiagnostics = await waitForDiagnosticItems(
                api,
                helperUri,
                () => true,
                "expected diagnostics report for helper.ds after edit",
            );

            // assert diagnostics requests kept responding with array payloads
            assert.ok(Array.isArray(baselineLibDiagnostics), "expected baseline lib diagnostics");
            assert.ok(Array.isArray(invalidLibDiagnostics), "expected invalid lib diagnostics");
            assert.ok(Array.isArray(restoredLibDiagnostics), "expected restored lib diagnostics");
            assert.ok(
                Array.isArray(baselineHelperDiagnostics),
                "expected baseline helper diagnostics",
            );
            assert.ok(
                Array.isArray(updatedHelperDiagnostics),
                "expected updated helper diagnostics",
            );
        } finally {
            // clean temporary files after each test run
            removeDirectory(project.path);
        }
    });

    test("handles diagnostics across a multi-file multi-edit flow", async () => {
        // create a workspace local multi-file project
        const project = createWorkspaceProject("destack-vscode-real-diag-flow-", {
            "main.ds": `import { value } from "./lib";
import { helper } from "./helper";
const output = helper(value);
`,
            "lib.ds": BASELINE_LIB_TEXT,
            "helper.ds": BASELINE_HELPER_TEXT,
        });

        try {
            // resolve uris and API
            const api = getDestackTestingApi();
            const mainUri = vscode.Uri.file(path.join(project.path, "main.ds"));
            const libUri = vscode.Uri.file(path.join(project.path, "lib.ds"));
            const helperUri = vscode.Uri.file(path.join(project.path, "helper.ds"));

            // execute round one disk edits across all project files
            writeFileTextByUri(
                mainUri,
                `import { value } from "./lib";
import { helper } from "./helper";
const output = helper(value + 1);
`,
            );
            writeFileTextByUri(libUri, UPDATED_LIB_TEXT);
            writeFileTextByUri(helperUri, UPDATED_HELPER_TEXT);

            // verify diagnostics requests settle for all three files in round one
            const mainRoundOneDiagnostics = await waitForDiagnosticItems(
                api,
                mainUri,
                () => true,
                "expected main.ds diagnostics after round one edits",
            );
            const libRoundOneDiagnostics = await waitForDiagnosticItems(
                api,
                libUri,
                () => true,
                "expected lib.ds diagnostics after round one edits",
            );
            const helperRoundOneDiagnostics = await waitForDiagnosticItems(
                api,
                helperUri,
                () => true,
                "expected helper.ds diagnostics after round one edits",
            );

            // execute round two disk edits across all project files
            writeFileTextByUri(
                mainUri,
                `import { value } from "./lib";
import { helper } from "./helper";
const output = helper(value + 2);
`,
            );
            writeFileTextByUri(libUri, "export const value = 3;\n");
            writeFileTextByUri(
                helperUri,
                `export function helper(input: number): number {
    const output = input + 2;
    return output;
}
`,
            );

            // verify diagnostics requests settle for all three files in round two
            const mainRoundTwoDiagnostics = await waitForDiagnosticItems(
                api,
                mainUri,
                () => true,
                "expected main.ds diagnostics after round two edits",
            );
            const libRoundTwoDiagnostics = await waitForDiagnosticItems(
                api,
                libUri,
                () => true,
                "expected lib.ds diagnostics after round two edits",
            );
            const helperRoundTwoDiagnostics = await waitForDiagnosticItems(
                api,
                helperUri,
                () => true,
                "expected helper.ds diagnostics after round two edits",
            );

            // assert all diagnostics responses keep array shape through both rounds
            assert.ok(
                Array.isArray(mainRoundOneDiagnostics),
                "expected main round-one diagnostics",
            );
            assert.ok(Array.isArray(libRoundOneDiagnostics), "expected lib round-one diagnostics");
            assert.ok(
                Array.isArray(helperRoundOneDiagnostics),
                "expected helper round-one diagnostics",
            );
            assert.ok(
                Array.isArray(mainRoundTwoDiagnostics),
                "expected main round-two diagnostics",
            );
            assert.ok(Array.isArray(libRoundTwoDiagnostics), "expected lib round-two diagnostics");
            assert.ok(
                Array.isArray(helperRoundTwoDiagnostics),
                "expected helper round-two diagnostics",
            );
        } finally {
            // clean temporary files after each test run
            removeDirectory(project.path);
        }
    });
});
