import assert from "node:assert/strict";
import test from "node:test";

import { Document } from "../dist/index.js";
import { openWorkspace, workspaces } from "./session.mjs";

for (const [name, open] of workspaces) {
    test(`${name}: formats source text`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
        ]);
        const workspace = fixture.workspace;
        const request = {
            document: Document.text("src/index.ds", "export  const value=1;"),
        };

        // format ad hoc source text
        const output = workspace.format(workspace.revision(), request);

        assert.equal(output.text, "export const value = 1;\n");
    });
}
