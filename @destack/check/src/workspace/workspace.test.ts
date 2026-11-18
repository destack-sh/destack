import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { join } from "node:path";
import { Workspace } from "./workspace.ts";

test("find no workspace outside any lockfile", async () => {
    // open an empty directory in a world
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const directory = await files.makeTemporaryDirectory("workspace-");

    expect(await Workspace.read(directory, files)).toBeUndefined();
});

test("read a workspace's members from its lockfile and walk their workspace dependencies both ways", async () => {
    // record a root on the library, the library, an app on it, a fixture inside the app and a tool on the app
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const root = await files.makeTemporaryDirectory("workspace-");
    await files.writeTextFile(
        join(root, "bun.lock"),
        `{
  "lockfileVersion": 1,
  "workspaces": {
    "": { "name": "@acme/workspace", "devDependencies": { "@acme/library": "workspace:*" }, },
    "packages/library": { "name": "@acme/library", "dependencies": { "uuid": "14.0.2" }, },
    "packages/app": { "name": "@acme/app", "dependencies": { "@acme/library": "workspace:*" }, },
    "packages/app/fixture": { "name": "@acme/fixture", },
    "tools/check": { "name": "@acme/check", "devDependencies": { "@acme/app": "workspace:^" }, },
  },
  "packages": {},
}
`,
    );
    await files.mkdir(join(root, "packages/app/src"), { recursive: true });

    // read from a directory inside a member and walk from the library and the app
    const workspace = await Workspace.read(join(root, "packages/app/src"), files);
    if (workspace === undefined) {
        throw new TypeError("the fixture has a lockfile");
    }
    expect({
        root: workspace.root,
        members: workspace.members,
        dependents: workspace.dependents("packages/library").toSorted(),
        dependencies: workspace.dependencies("tools/check").toSorted(),
        unrelated: workspace.dependents("packages/missing"),
        affectedByApp: workspace.affected(["packages/app/src/app.ts", "README.md"]),
        affectedByLibrary: workspace.affected(["packages/library/package.json"]),
        affectedByFixture: workspace.affected(["packages/app/fixture/package.json"]),
        affectedByRoot: workspace.affected(["packages/app/src/app.ts", "bun.lock"]),
    }).toEqual({
        root,
        members: [
            { directory: ".", name: "@acme/workspace", dependencies: ["packages/library"] },
            { directory: "packages/library", name: "@acme/library", dependencies: [] },
            {
                directory: "packages/app",
                name: "@acme/app",
                dependencies: ["packages/library"],
            },
            { directory: "packages/app/fixture", name: "@acme/fixture", dependencies: [] },
            { directory: "tools/check", name: "@acme/check", dependencies: ["packages/app"] },
        ],
        dependents: [".", "packages/app", "packages/library", "tools/check"],
        dependencies: ["packages/app", "packages/library", "tools/check"],
        unrelated: [],
        affectedByApp: ["packages/app", "tools/check"],
        affectedByLibrary: ["packages/app", "packages/library", "tools/check"],
        affectedByFixture: ["packages/app", "packages/app/fixture", "tools/check"],
        affectedByRoot: undefined,
    });
});

test("select a member's existing changed modules, leaving out deleted ones, other files and nested members' modules", async () => {
    // record a workspace with an app and a fixture package nested in it
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const root = await files.makeTemporaryDirectory("workspace-");
    await files.writeTextFile(
        join(root, "bun.lock"),
        `{
  "lockfileVersion": 1,
  "workspaces": {
    "": { "name": "@acme/workspace" },
    "packages/app": { "name": "@acme/app" },
    "packages/app/fixture": { "name": "@acme/fixture" },
  },
  "packages": {},
}
`,
    );
    await files.mkdir(join(root, "packages/app/src"), { recursive: true });
    await files.mkdir(join(root, "packages/app/fixture"), { recursive: true });
    for (const path of [
        "packages/app/src/kept.ts",
        "packages/app/src/view.tsx",
        "packages/app/fixture/index.ts",
    ]) {
        await files.writeTextFile(join(root, path), "export {};\n");
    }

    // select the app's modules among changes to its modules, a deleted one, its manifest and its fixture
    const workspace = await Workspace.read(root, files);
    if (workspace === undefined) {
        throw new TypeError("the fixture has a lockfile");
    }
    const modules = await workspace.modules(
        join(root, "packages/app"),
        [
            "packages/app/src/kept.ts",
            "packages/app/src/view.tsx",
            "packages/app/src/deleted.ts",
            "packages/app/package.json",
            "packages/app/fixture/index.ts",
            "README.md",
        ],
        files,
    );
    expect(modules).toEqual(["src/kept.ts", "src/view.tsx"]);
});
