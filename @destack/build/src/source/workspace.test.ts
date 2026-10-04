import { expect, onTestFinished, test } from "@destack/test";
import { mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Workspace } from "./workspace.ts";

test("find no workspace outside any lockfile", async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-workspace-")));
    onTestFinished(() => rm(directory, { recursive: true }));

    expect(await Workspace.read(directory)).toBeUndefined();
});

test("read a workspace's members from its lockfile and walk their workspace dependencies both ways", async () => {
    // record a root, a library, an app on the library and a tool on the app, with trailing commas
    const root = await realpath(await mkdtemp(join(tmpdir(), "destack-workspace-")));
    onTestFinished(() => rm(root, { recursive: true }));
    await writeFile(
        join(root, "bun.lock"),
        `{
  "lockfileVersion": 1,
  "workspaces": {
    "": { "name": "@acme/workspace", "devDependencies": { "typescript": "7.0.2" }, },
    "packages/library": { "name": "@acme/library", "dependencies": { "uuid": "14.0.2" }, },
    "packages/app": { "name": "@acme/app", "dependencies": { "@acme/library": "workspace:*" }, },
    "tools/check": { "name": "@acme/check", "devDependencies": { "@acme/app": "workspace:^" }, },
  },
  "packages": {},
}
`,
    );
    await mkdir(join(root, "packages/app/src"), { recursive: true });

    // read from a directory inside a member, then walk from the library and the app
    const workspace = await Workspace.read(join(root, "packages/app/src"));
    if (workspace === undefined) {
        throw new TypeError("the fixture has a lockfile");
    }
    expect({
        root: workspace.root,
        members: workspace.members,
        dependents: workspace.dependents("packages/library").toSorted(),
        dependencies: workspace.dependencies("tools/check").toSorted(),
        unrelated: workspace.dependents("packages/missing"),
    }).toEqual({
        root,
        members: [
            { directory: ".", name: "@acme/workspace", dependencies: [] },
            { directory: "packages/library", name: "@acme/library", dependencies: [] },
            {
                directory: "packages/app",
                name: "@acme/app",
                dependencies: ["packages/library"],
            },
            { directory: "tools/check", name: "@acme/check", dependencies: ["packages/app"] },
        ],
        dependents: ["packages/app", "packages/library", "tools/check"],
        dependencies: ["packages/app", "packages/library", "tools/check"],
        unrelated: [],
    });
});
