import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { present, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { buildPackage, type PackageBuild } from "../src/index.ts";
import { loadExtensions } from "../src/compile/extension.ts";
import { readPackageJson } from "../src/source/index.ts";

/** The schema every package definition names. */
const SCHEMA = "https://destack.app/schemas/2026.9.0/destack.json";

/** The extension the leaf package declares, rewriting a marker in every module it transforms. */
const LEAF_EXTENSION = `/** Rewrite the marker in each module. */
export const leafExtension = {
    transform: () => [
        {
            name: "@fixture/leaf",
            transform: (code) => ({ code: code.replaceAll("unchanged", "transformed"), map: null }),
        },
    ],
};
`;

/** The leaf's extension, emitting a generated entry that imports the leaf. */
const GLUE_EXTENSION = `/** Emit the generated glue as an entry of each compilation. */
export const leafExtension = {
    compile: (compilation) => {
        compilation.entry("glue", "virtual:@fixture/leaf/glue");

        return [
            {
                name: "@fixture/leaf",
                resolveId: (id) => (id === "virtual:@fixture/leaf/glue" ? "\\0" + id : null),
                load: (id) =>
                    id === "\\0virtual:@fixture/leaf/glue" ? 'export * from "@fixture/leaf";' : null,
            },
        ];
    },
};
`;

/** Write a package's manifest, definition and modules into a directory. */
async function writePackage(
    directory: string,
    manifest: Readonly<Record<string, unknown>>,
    definition: Readonly<Record<string, unknown>>,
    modules: Readonly<Record<string, string>>,
): Promise<void> {
    await mkdir(join(directory, "src"), { recursive: true });
    await writeFile(
        join(directory, "package.json"),
        JSON.stringify({ version: "2026.9.0", type: "module", ...manifest }),
    );
    await writeFile(
        join(directory, "destack.json"),
        JSON.stringify({
            $schema: SCHEMA,
            language: "typescript",
            runtimes: ["bun"],
            ...definition,
        }),
    );
    for (const [path, text] of Object.entries(modules)) {
        await writeFile(join(directory, "src", path), text);
    }
}

/** Link a package into a dependent's node_modules, as a workspace installs it. */
async function link(root: string, dependent: string, name: string): Promise<void> {
    await mkdir(join(root, dependent, "node_modules", "@fixture"), { recursive: true });
    await symlink(join(root, name), join(root, dependent, "node_modules", "@fixture", name));
}

/** Run an entry of a build's Bun output, reading its exports. */
async function readEntry(build: PackageBuild, name: string): Promise<Record<string, unknown>> {
    const path = present(build.reader.manifest.outputs["bun"]?.exports[name], `entry ${name}`);

    return schema
        .record(schema.string(), schema.unknown())
        .parse(await import(join(build.directory, path)));
}

/** Write an application whose two packages require one extension leaf, returning its root. */
async function writeApplication(): Promise<string> {
    // write the leaf declaring the extension, and two packages requiring it
    const root = await realpath(await mkdtemp(join(tmpdir(), "destack-build-extension-")));
    await writeFile(join(root, "bun.lock"), JSON.stringify({ lockfileVersion: 1, workspaces: {} }));
    await writePackage(
        join(root, "leaf"),
        { name: "@fixture/leaf", exports: { ".": "./src/index.ts", "./build": "./src/build.ts" } },
        {
            id: "package-01a10b00-0000-7000-8000-000000000001",
            exports: { "./build": { runtimes: ["bun"] } },
            build: "./build#leafExtension",
        },
        { "index.ts": "export {};\n", "build.ts": LEAF_EXTENSION },
    );
    for (const [index, name] of ["middle", "other"].entries()) {
        await writePackage(
            join(root, name),
            {
                name: `@fixture/${name}`,
                exports: { ".": "./src/index.ts" },
                dependencies: { "@fixture/leaf": "2026.9.0" },
            },
            { id: `package-01a10b00-0000-7000-8000-00000000000${index + 2}` },
            { "index.ts": "export {};\n" },
        );
        await link(root, name, "leaf");
    }

    // write the application requiring both packages but not the leaf
    await writePackage(
        join(root, "application"),
        {
            name: "@fixture/application",
            exports: { ".": "./src/index.ts" },
            dependencies: { "@fixture/middle": "2026.9.0", "@fixture/other": "2026.9.0" },
        },
        { id: "package-01a10b00-0000-7000-8000-000000000004" },
        {
            "index.ts": 'export * from "./marker.ts";\n',
            "marker.ts":
                '/** The marker the leaf rewrites. */\nexport const marker = "unchanged";\n',
        },
    );
    await link(root, "application", "middle");
    await link(root, "application", "other");

    return root;
}

test("apply the extension of a transitive dependency once in a build of a package requiring it indirectly", async () => {
    // load the application's extensions and build its server module
    const root = await writeApplication();
    const directory = join(root, "application");
    try {
        const loaded = await loadExtensions(directory, await readPackageJson(directory));
        await using build = await buildPackage({
            directory,
            dependencies: {},
            outputs: { bun: { kind: "module", runtime: "bun", bundle: true } },
        });

        // load the leaf's extension once and compile the application's marker through it
        expect([loaded.map((entry) => entry.directory), await readEntry(build, ".")]).toEqual([
            [await realpath(join(root, "leaf"))],
            { marker: "transformed" },
        ]);
    } finally {
        await rm(root, { recursive: true });
    }
});

test("load a development dependency's own extension in development without walking its dependencies", async () => {
    // write a tool requiring the leaf, declaring its own extension, which the application requires in development
    const root = await writeApplication();
    await writePackage(
        join(root, "tool"),
        {
            name: "@fixture/tool",
            exports: { ".": "./src/index.ts", "./build": "./src/build.ts" },
            dependencies: { "@fixture/leaf": "2026.9.0" },
        },
        {
            id: "package-01a10b00-0000-7000-8000-000000000005",
            exports: { "./build": { runtimes: ["bun"] } },
            build: "./build#toolExtension",
        },
        { "index.ts": "export {};\n", "build.ts": "export const toolExtension = {};\n" },
    );
    await link(root, "tool", "leaf");
    await writePackage(
        join(root, "site"),
        {
            name: "@fixture/site",
            exports: { ".": "./src/index.ts" },
            devDependencies: { "@fixture/tool": "2026.9.0" },
        },
        { id: "package-01a10b00-0000-7000-8000-000000000006" },
        { "index.ts": "export {};\n" },
    );
    await link(root, "site", "tool");
    const directory = join(root, "site");
    try {
        // load the site's extensions in development and in a release build
        const packageJson = await readPackageJson(directory);
        const development = await loadExtensions(directory, packageJson, { development: true });
        const release = await loadExtensions(directory, packageJson);
        expect([
            development.map((entry) => entry.directory),
            release.map((entry) => entry.directory),
        ]).toEqual([[await realpath(join(root, "tool"))], []]);
    } finally {
        await rm(root, { recursive: true });
    }
});

test("bundle what a module an extension generates imports, though the package does not declare it", async () => {
    // let the leaf's extension emit a generated entry importing the leaf, which the application does not declare
    const root = await writeApplication();
    const directory = join(root, "application");
    await writeFile(join(root, "leaf", "src", "build.ts"), GLUE_EXTENSION);
    await writeFile(join(root, "leaf", "src", "index.ts"), 'export const glued = "glued";\n');
    await link(root, "application", "leaf");
    try {
        // build the application's server module through the extension
        await using build = await buildPackage({
            directory,
            dependencies: {},
            outputs: { bun: { kind: "module", runtime: "bun", bundle: true } },
        });

        // run the generated entry, which carries the leaf's code it imports
        expect(await readEntry(build, "glue")).toEqual({ glued: "glued" });
    } finally {
        await rm(root, { recursive: true });
    }
});
