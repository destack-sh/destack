import assert from "node:assert/strict";

import * as vscode from "vscode";

import {
    DESTACK_COMMANDS,
    assertDefinitionLocation,
    definitionLocation,
    ensureBridgeReady,
    fixtureDocumentUri,
    getDestackTestingApi,
    openDocument,
} from "./tests";

suite("bridge", () => {
    suiteSetup(async () => {
        // wait for extension activation before tests run
        await ensureBridgeReady();
    });

    test("test_activates_extension_and_registers_commands", async () => {
        // fetch registered commands from vscode
        const commands = await vscode.commands.getCommands(true);

        // assert all expected extension commands are present
        for (const command of DESTACK_COMMANDS) {
            assert.ok(commands.includes(command), `missing command: ${command}`);
        }
    });

    test("test_exports_runtime_testing_api", async () => {
        // resolve and validate the exported extension testing api
        const api = getDestackTestingApi();
        assert.equal(typeof api.sendRequestForTests, "function");
        assert.equal(typeof api.sendNotificationForTests, "function");
        assert.equal(typeof api.getRuntimeStateForTests, "function");
    });

    test("test_routes_definition_provider_through_language_client", async () => {
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
            "definition provider did not resolve to the answer declaration",
        );
    });
});
