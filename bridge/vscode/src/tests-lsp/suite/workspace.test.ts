import assert from "node:assert/strict";

import {
    ensureFixtureReady,
    getDestackTestingApi,
    isRealServerMode,
    seedDiagnosticLoad,
    withTimeout,
    workspaceRootPath,
} from "./tests";

/** Timeout for workspace command and diagnostic completion checks. */
const WORKSPACE_DIAGNOSTIC_TIMEOUT_MILLISECONDS = 20_000;

suite("lsp.workspace", () => {
    suiteSetup(async function () {
        // skip this suite outside real server mode
        if (!isRealServerMode()) {
            this.skip();
        }

        // wait for extension activation before tests run
        await ensureFixtureReady();
    });

    test("keeps workspace diagnostics responsive after command bursts", async () => {
        // create enough files to exercise workspace diagnostics under load
        seedDiagnosticLoad(workspaceRootPath(), 40);

        // run several workspace commands before diagnostics
        const api = getDestackTestingApi();
        await requestWorkspaceCommand(api, "destack.rescan");
        await requestWorkspaceCommand(api, "destack.reindex");
        await requestWorkspaceCommand(api, "destack.clearCache");

        // request workspace diagnostics and require completion
        const report = await withTimeout(
            api.sendRequestForTests<unknown>("workspace/diagnostic", {
                identifier: null,
                previousResultIds: [],
                workDoneProgressParams: { workDoneToken: null },
                partialResultParams: { partialResultToken: null },
            }),
            WORKSPACE_DIAGNOSTIC_TIMEOUT_MILLISECONDS,
            "workspace diagnostics completion timed out",
        );
        assert.notEqual(report, undefined, "workspace diagnostics request should complete");
    });
});

/** Request one workspace command and require completion. */
async function requestWorkspaceCommand(
    api: ReturnType<typeof getDestackTestingApi>,
    command: string,
): Promise<void> {
    await withTimeout(
        api.sendRequestForTests("workspace/executeCommand", {
            command,
            arguments: [],
        }),
        WORKSPACE_DIAGNOSTIC_TIMEOUT_MILLISECONDS,
        `${command} request timed out`,
    );
}
