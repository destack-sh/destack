import assert from "node:assert/strict";
import test from "node:test";

import { BuildRequest } from "../dist/index.js";
import { openWorkspace, workspaces } from "./session.mjs";

for (const [name, open] of workspaces) {
    test(`${name}: builds a JavaScript bundle`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const value = 1;"],
        ]);
        const workspace = fixture.workspace;
        const revision = workspace.revision();
        const module = workspace.module("src/index.ds");
        const target = workspace.target(revision, module.id.package, "js");

        // build the package target through the public bridge
        const output = workspace.build(revision, BuildRequest.target(target));
        const file = output.bundle.files[0];

        assert.equal(output.kind, "bundle");
        assert.equal(output.bundle.emit, "js");
        assert.equal(output.bundle.mode, "singleFile");
        assert.equal(output.bundle.files.length, 1);
        assert.equal(file.section, "module");
        assert.equal(fixture.relativeUri(file.uri), "dist/js.js");
        assert.equal(file.fileType, "javaScript");
        assert.equal(file.source, undefined);
    });
}
