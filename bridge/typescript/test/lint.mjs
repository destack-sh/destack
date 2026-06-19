import assert from "node:assert/strict";
import test from "node:test";

import { Scope } from "../dist/index.js";
import { openWorkspace, workspaces } from "./session.mjs";

for (const [name, open] of workspaces) {
    test(`${name}: lints a clean module`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const value = 1;"],
        ]);
        const workspace = fixture.workspace;
        const revision = workspace.revision();
        const module = workspace.module("src/index.ds");
        const profile = workspace.profile(revision, module, "js");

        // lint the loaded module profile
        const output = workspace.lint(revision, { scope: Scope.module(module, profile) });

        assert.deepEqual(output.diagnostics, []);
    });
}
