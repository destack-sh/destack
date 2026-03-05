import * as path from "node:path";
import * as vscode from "vscode";

import {
    assertDefinitionLocation,
    createWorkspaceProject,
    definitionValueExportFixture,
    definitionLocationFromRange,
    ensureFixtureReady,
    getDestackTestingApi,
    isRealServerMode,
    openDocument,
    removeDirectory,
    withTimeout,
    writeFileTextByUri,
} from "./tests";

/** Timeout for workspace command requests in real-server LSP tests. */
const COMMAND_TIMEOUT_MILLISECONDS = 20_000;
suite("lsp.commands", function () {
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
        // load canonical request and target range values
        const definitionFixture = definitionValueExportFixture();

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
            await assertDefinitionLocation(
                api,
                mainUri,
                definitionFixture.requestPosition,
                definitionLocationFromRange(libUri, definitionFixture.expectedDefinitionRange),
                `definition request did not resolve to exact value export in ${libUri.toString()}`,
            );

            // execute rescan and verify definition behavior again
            await requestWorkspaceCommand(api, "destack.rescan");
            await assertDefinitionLocation(
                api,
                mainUri,
                definitionFixture.requestPosition,
                definitionLocationFromRange(libUri, definitionFixture.expectedDefinitionRange),
                `definition request did not resolve to exact value export in ${libUri.toString()}`,
            );

            // apply edits, execute reindex, and verify response behavior
            writeFileTextByUri(libUri, "export const value = 2;\n");
            await requestWorkspaceCommand(api, "destack.reindex");
            await assertDefinitionLocation(
                api,
                mainUri,
                definitionFixture.requestPosition,
                definitionLocationFromRange(libUri, definitionFixture.expectedDefinitionRange),
                `definition request did not resolve to exact value export in ${libUri.toString()}`,
            );

            // apply another edit, clear cache, and verify response behavior
            writeFileTextByUri(libUri, "export const value = 3;\n");
            await requestWorkspaceCommand(api, "destack.clearCache");
            await assertDefinitionLocation(
                api,
                mainUri,
                definitionFixture.requestPosition,
                definitionLocationFromRange(libUri, definitionFixture.expectedDefinitionRange),
                `definition request did not resolve to exact value export in ${libUri.toString()}`,
            );
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
