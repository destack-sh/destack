import assert from "node:assert/strict";
import test from "node:test";

import { openWorkspace, workspaces } from "./session.mjs";

for (const [name, open] of workspaces) {
    test(`${name}: checks a valid module`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const value = 1;"],
        ]);
        const workspace = fixture.workspace;
        const revision = workspace.revision();
        const module = workspace.module("src/index.ds");
        const profile = workspace.profile(revision, module, "js");

        // check the module under the selected target profile
        const output = workspace.check(revision, module, profile);

        assert.deepEqual(output.diagnostics, []);
    });

    test(`${name}: reports a missing type annotation`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "const values = [];"],
        ]);
        const workspace = fixture.workspace;
        const revision = workspace.revision();
        const module = workspace.module("src/index.ds");
        const profile = workspace.profile(revision, module, "js");

        // check the module and read its type diagnostics
        const output = workspace.check(revision, module, profile);

        assert.equal(output.diagnostics.length, 2);
        assert.equal(output.diagnostics[0].code, "EC101");
        assert.equal(output.diagnostics[0].severity, "error");
        assert.equal(output.diagnostics[0].message, "missing type annotation");
        assert.equal(output.diagnostics[1].code, "EC101");
        assert.equal(output.diagnostics[1].severity, "error");
        assert.equal(output.diagnostics[1].message, "missing type annotation");
    });
}
