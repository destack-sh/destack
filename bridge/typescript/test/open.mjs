import assert from "node:assert/strict";
import test from "node:test";

import { openRepository, openWorkspace, repositories, workspaces } from "./session.mjs";

for (const [name, open] of workspaces) {
    test(`${name}: opens a memory source`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const value = 1;"],
        ]);
        const workspace = fixture.workspace;

        // list seeded source files
        assert.deepEqual(
            workspace.files().map((file) => file.path),
            ["destack.json", "src/index.ds"],
        );
    });
}

for (const [name, open] of repositories) {
    test(`${name}: opens a repository workspace`, async () => {
        const fixture = await openRepository(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const value = 1;"],
        ]);
        const workspace = fixture.repository.workspace();

        // list seeded source files through the repository workspace
        assert.deepEqual(
            workspace.files().map((file) => file.path),
            ["destack.json", "src/index.ds"],
        );
    });
}
