import assert from "node:assert/strict";
import test from "node:test";

import { Edit } from "../dist/index.js";
import { openWorkspace, workspaces } from "./session.mjs";

for (const [name, open] of workspaces) {
    test(`${name}: applies a source edit`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const value = 1;"],
        ]);
        const workspace = fixture.workspace;

        // apply one bridge edit
        const commit = workspace.edit([
            Edit.setText("src/next.ds", "export const next = 2;"),
        ]);

        // require a changed revision and file list
        assert.notEqual(commit.before.id, commit.after.id);
        assert.deepEqual(
            workspace.files().map((file) => file.path),
            ["destack.json", "src/index.ds", "src/next.ds"],
        );
    });
}
