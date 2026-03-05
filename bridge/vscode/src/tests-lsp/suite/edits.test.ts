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
    replaceDocumentText,
} from "./tests";
suite("lsp.edits", function () {
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
        // load canonical request and target range values
        const definitionFixture = definitionValueExportFixture();

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
            await assertDefinitionLocation(
                api,
                mainUri,
                definitionFixture.requestPosition,
                definitionLocationFromRange(libUri, definitionFixture.expectedDefinitionRange),
                `definition request did not resolve to exact value export in ${libUri.toString()}`,
            );

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
            await assertDefinitionLocation(
                api,
                mainUri,
                definitionFixture.requestPosition,
                definitionLocationFromRange(libUri, definitionFixture.expectedDefinitionRange),
                `definition request did not resolve to exact value export in ${libUri.toString()}`,
            );

            // apply another round of edits and verify definition target again
            await replaceDocumentText(
                mainDocument,
                `import { value } from "./lib";
function finalRender(input: number): number {
    return input + value;
}
const output = finalRender(3);
`,
            );
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
