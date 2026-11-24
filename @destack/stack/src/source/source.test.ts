import { expect, test } from "@destack/test";
import { format } from "oxfmt";
import { PackageSource } from "./source.ts";

/** The formatter options of the workspace the fixture package lives in. */
const FORMAT = { tabWidth: 4, printWidth: 100, trailingComma: "all" } as const;

/** A stack package: a manifest, a root module re-exporting the stack, and the stack's module. */
const FILES: Readonly<Record<string, string>> = {
    "package.json": `{
    "name": "@ada/team",
    "exports": {
        ".": "./src/index.ts"
    },
    "dependencies": {
        "@destack/stack": "workspace:*",
        "@destack/vault": "latest"
    }
}
`,
    "src/index.ts": `export * from "./stack/index.ts";\n`,
    "src/stack/index.ts": `export { SPACE_STACK } from "./space.ts";\n`,
    "src/stack/space.ts": `import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";
import vaultPackage from "@destack/vault/package";

/** The stack of the team's space. */
export const SPACE_STACK = defineStack({
    types: [vaultPackage],
    installations: {
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
    },
});
`,
};

/** Edit the fixture package, answering its changed files. */
async function edited(
    operations: Parameters<PackageSource["apply"]>[0][],
): Promise<Record<string, string>> {
    const source = new PackageSource(
        async (path) => {
            const text = FILES[path];
            if (text === undefined) {
                throw new Error(`file '${path}' not found`);
            }

            return text;
        },
        async (path, text) => (await format(path, text, FORMAT)).code,
    );
    for (const operation of operations) {
        await source.apply(operation);
    }

    return Object.fromEntries(await source.changed());
}

test("install a package in a stack behind re-exports, importing it and depending on it", async () => {
    const changed = await edited([
        { kind: "dependency.add", package: "@ada/task-list", range: "latest" },
        {
            kind: "installation.add",
            stack: "SPACE_STACK",
            name: "tasks",
            package: "@ada/task-list",
        },
    ]);

    expect(changed).toEqual({
        "package.json": `{
    "name": "@ada/team",
    "exports": {
        ".": "./src/index.ts"
    },
    "dependencies": {
        "@ada/task-list": "latest",
        "@destack/stack": "workspace:*",
        "@destack/vault": "latest"
    }
}
`,
        "src/stack/space.ts": `import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";
import vaultPackage from "@destack/vault/package";
import taskListPackage from "@ada/task-list/package";

/** The stack of the team's space. */
export const SPACE_STACK = defineStack({
    types: [vaultPackage],
    installations: {
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
        tasks: install(taskListPackage, {}, { name: "tasks" }),
    },
});
`,
    });
});

test("uninstall a package, dropping its import once unread and its dependency", async () => {
    // install and then remove tasks, which leaves the files as they were
    const changed = await edited([
        { kind: "dependency.add", package: "@ada/task-list", range: "latest" },
        {
            kind: "installation.add",
            stack: "SPACE_STACK",
            name: "tasks",
            package: "@ada/task-list",
        },
        { kind: "installation.remove", stack: "SPACE_STACK", name: "tasks" },
        { kind: "dependency.remove", package: "@ada/task-list" },
    ]);

    expect(changed).toEqual({
        "package.json": FILES["package.json"],
        "src/stack/space.ts": FILES["src/stack/space.ts"],
    });
});

test("keep an import another part of the module still reads", async () => {
    // remove vault, whose package the stack's types still list
    const changed = await edited([
        { kind: "installation.remove", stack: "SPACE_STACK", name: "vault" },
    ]);

    expect(changed).toEqual({
        "src/stack/space.ts": `import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";
import vaultPackage from "@destack/vault/package";

/** The stack of the team's space. */
export const SPACE_STACK = defineStack({
    types: [vaultPackage],
    installations: {},
});
`,
    });
});

test("refuse a taken installation name, a missing installation, dependency or stack", async () => {
    const refusals = await Promise.all(
        [
            edited([
                {
                    kind: "installation.add",
                    stack: "SPACE_STACK",
                    name: "vault",
                    package: "@ada/vault",
                },
            ]),
            edited([{ kind: "installation.remove", stack: "SPACE_STACK", name: "tasks" }]),
            edited([{ kind: "dependency.remove", package: "@ada/task-list" }]),
            edited([{ kind: "installation.remove", stack: "HOME_STACK", name: "vault" }]),
        ].map((edit) => edit.then(() => undefined).catch((error: Error) => error.message)),
    );

    expect(refusals).toEqual([
        "installation 'vault' already exists",
        "installation 'tasks' not found",
        "dependency '@ada/task-list' not found",
        "stack 'HOME_STACK' not found",
    ]);
});
