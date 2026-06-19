import assert from "node:assert/strict";
import test from "node:test";

import { openWorkspace, workspaces } from "./session.mjs";

for (const [name, open] of workspaces) {
    test(`${name}: parses a clean module`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const value = 1;"],
        ]);
        const workspace = fixture.workspace;
        const revision = workspace.revision();
        const module = workspace.module("src/index.ds");

        // parse one loaded source module
        const output = workspace.parse(revision, module);

        assert.deepEqual(output.diagnostics, []);
    });

    test(`${name}: reports a parse syntax error`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const broken ="],
        ]);
        const workspace = fixture.workspace;
        const revision = workspace.revision();
        const module = workspace.module("src/index.ds");

        // parse and inspect the returned syntax diagnostic
        const output = workspace.parse(revision, module);

        assert.equal(output.diagnostics.length, 1);
        assert.equal(output.diagnostics[0].code, "EP001");
        assert.equal(output.diagnostics[0].severity, "error");
        assert.equal(output.diagnostics[0].message, "parse error: unexpected End in Declarator");
    });
}
