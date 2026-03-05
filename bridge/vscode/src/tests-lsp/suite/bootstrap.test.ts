import assert from "node:assert/strict";
import * as vscode from "vscode";

import {
    DESTACK_COMMANDS,
    assertDefinitionLocation,
    definitionLocation,
    ensureFixtureReady,
    fixtureDocumentUri,
    getDestackTestingApi,
    openDocument,
} from "./tests";

suite("lsp.bootstrap", () => {
    suiteSetup(async () => {
        // wait for extension activation before tests run
        await ensureFixtureReady();
    });

    test("activates and registers commands", async () => {
        // fetch registered commands from vscode
        const commands = await vscode.commands.getCommands(true);

        // assert all expected extension commands are present
        for (const command of DESTACK_COMMANDS) {
            assert.ok(commands.includes(command), `missing command: ${command}`);
        }
    });

    test("serves definition requests", async () => {
        // resolve fixture uri and open the source document
        const fixtureUri = fixtureDocumentUri();
        const api = getDestackTestingApi();
        await openDocument(fixtureUri);

        // assert exact definition location for `answer` at call site
        await assertDefinitionLocation(
            api,
            fixtureUri,
            { line: 4, character: 16 },
            definitionLocation(fixtureUri, 0, 16, 0, 22),
            "definition request did not resolve to exact answer declaration",
        );
    });
});
