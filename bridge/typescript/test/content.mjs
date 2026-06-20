import assert from "node:assert/strict";
import test from "node:test";

import { ArtifactKey } from "../dist/index.js";
import { openWorkspace, workspaces } from "./session.mjs";

for (const [name, open] of workspaces) {
    test(`${name}: reads source content dependencies`, async () => {
        const fixture = await openWorkspace(open, [
            ["destack.json", '{"name":"@test/app"}'],
            ["src/index.ds", "export const value = 1;"],
        ]);
        const workspace = fixture.workspace;
        const revision = workspace.revision();
        const module = workspace.module("src/index.ds");
        const key = ArtifactKey.dirParsed(module.id);

        // require the parsed artifact that records source content dependencies
        const record = workspace.artifactRecord(revision, key);

        // read the exact file content dependency through the public content API
        const fileContents = record.dependencies
            .filter((dependency) => dependency.kind === "source")
            .map((dependency) => workspace.text(dependency.content));

        assert.deepEqual(fileContents, ["export const value = 1;"]);
    });
}
