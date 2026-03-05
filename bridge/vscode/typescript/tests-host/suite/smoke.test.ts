import assert from "node:assert/strict";
import * as vscode from "vscode";

import {
    DESTACK_COMMANDS,
    definitionLocations,
    ensureFixtureReady,
    fixtureDocumentUri,
    getDestackTestingApi,
    openDocument,
    waitForRequestResult,
} from "./support";

suite("destack extension host smoke", () => {
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

        // request definition and assert response shape
        const definition = await waitForRequestResult(
            api,
            "textDocument/definition",
            {
                textDocument: { uri: fixtureUri.toString() },
                position: { line: 5, character: 16 },
            },
            (result) => result !== undefined,
            "definition request did not complete",
        );

        // allow null when no target exists, otherwise require at least one location
        if (definition == null) {
            return;
        }

        const locations = definitionLocations(definition);
        assert.ok(locations.length > 0, "definition response should include a location payload");
    });
});
