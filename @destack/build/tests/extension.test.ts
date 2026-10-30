import type { FileSystem } from "@destack/environment";
import { testSimulation } from "../src/test/world.ts";
import { DependencyTree } from "../src/dependency/index.ts";
import { join } from "node:path";
import { present, schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { Compiler, PackageBuild, Workspace, CompilerPool } from "../src/index.ts";
import { loadExtensions } from "../src/compile/extension.ts";
import { readPackageJson } from "../src/source/index.ts";
import { LocalPackageStore } from "../src/store/local.ts";

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
const BARREL_EXTENSION = `/** Emit the generated barrel as an entry of each compilation. */
export const leafExtension = {
    compile: (compilation: { entry(name: string, id: string): void }) => {
        compilation.entry("barrel", "virtual:@fixture/leaf/barrel");

        return [
            {
                name: "@fixture/leaf",
                resolveId: (id: string) => (id === "virtual:@fixture/leaf/barrel" ? "\\0" + id : null),
                load: (id: string) =>
                    id === "\\0virtual:@fixture/leaf/barrel" ? 'export * from "@fixture/leaf";' : null,
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
    files: FileSystem,
): Promise<void> {
    await files.mkdir(join(directory, "src"), { recursive: true });
    await files.writeTextFile(
        join(directory, "package.json"),
        JSON.stringify({ version: "2026.9.0", type: "module", ...manifest }),
    );
    await files.writeTextFile(
        join(directory, "destack.json"),
        JSON.stringify({
            $schema: SCHEMA,
            language: "typescript",
            runtimes: ["bun"],
            ...definition,
        }),
    );
    for (const [path, text] of Object.entries(modules)) {
        await files.writeTextFile(join(directory, "src", path), text);
    }
}

/** Write a workspace's lockfile, listing its members by their directory names. */
async function writeLockfile(
    root: string,
    members: readonly string[],
    files: FileSystem,
): Promise<void> {
    const workspaces = Object.fromEntries(
        members.map((name) => [name, { name: `@fixture/${name}` }]),
    );
    await files.writeTextFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 2, workspaces, packages: {} }),
    );
}

/** Link a package into a dependent's node_modules, as a workspace installs it. */
async function link(
    root: string,
    dependent: string,
    name: string,
    files: FileSystem,
): Promise<void> {
    await files.mkdir(join(root, dependent, "node_modules", "@fixture"), { recursive: true });
    await files.symlink(join(root, name), join(root, dependent, "node_modules", "@fixture", name));
}

/** Run an entry of a build's Bun output, reading its exports. */
async function readEntry(build: PackageBuild, name: string): Promise<Record<string, unknown>> {
    const path = present(
        build.reader.manifest.outputs["bun"]?.exports[name]?.path,
        `entry ${name}`,
    );

    return schema
        .record(schema.string(), schema.unknown())
        .parse(await import(join(build.directory, path)));
}

/** Write an application whose two packages require one extension leaf, returning its root. */
async function writeApplication(files: FileSystem): Promise<string> {
    // write the leaf declaring the extension, and two packages requiring it
    const root = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-extension-"),
    );
    await writeLockfile(root, ["leaf", "middle", "other", "application"], files);
    await writePackage(
        join(root, "leaf"),
        { name: "@fixture/leaf", exports: { ".": "./src/index.ts", "./build": "./src/build.ts" } },
        {
            id: "package-01a10b00-0000-7000-8000-000000000001",
            exports: { "./build": { runtimes: ["bun"] } },
            build: "./build#leafExtension",
        },
        { "index.ts": "export {};\n", "build.ts": LEAF_EXTENSION },
        files,
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
            files,
        );
        await link(root, name, "leaf", files);
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
        files,
    );
    await link(root, "application", "middle", files);
    await link(root, "application", "other", files);

    return root;
}

/** Declare a dependency of a fixture package in one of its package.json groups. */
async function declare(
    directory: string,
    group: "dependencies" | "devDependencies",
    name: string,
    files: FileSystem,
): Promise<void> {
    const manifest = join(directory, "package.json");
    const declared = schema.looseObject({}).parse(JSON.parse(await files.readTextFile(manifest)));
    const existing = schema.record(schema.string(), schema.string()).parse(declared[group] ?? {});
    const updated = { ...existing, [name]: "2026.9.0" };
    await files.writeTextFile(manifest, JSON.stringify({ ...declared, [group]: updated }));
}

/** List the packages whose extensions compile a fixture package in its destack.json. */
async function listExtensions(
    directory: string,
    extensions: readonly string[],
    files: FileSystem,
): Promise<void> {
    const path = join(directory, "destack.json");
    const definition = schema.looseObject({}).parse(JSON.parse(await files.readTextFile(path)));
    await files.writeTextFile(path, JSON.stringify({ ...definition, extensions }));
}

test("apply the extensions of a package's runtime dependency closure, its dependencies' among them", async () => {
    // load the application's extensions and build its server module while only its dependencies require the leaf
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await writeApplication(files);
    const directory = join(root, "application");
    const outputs = { bun: { kind: "module", runtime: "bun" } } as const;
    const loaded = await loadExtensions(
        directory,
        await readPackageJson(directory, files),
        await DependencyTree.read({}, files),
        files,
    );
    await using store = await LocalPackageStore.temporary(environment);
    await using compiler = await Compiler.start(directory, { store });
    await using transformed = await compiler.build({ directory, dependencies: {}, outputs });
    expect([loaded.map((entry) => entry.directory), await readEntry(transformed, ".")]).toEqual([
        [await files.realPath(join(root, "leaf"))],
        { marker: "transformed" },
    ]);
});

test("compile a package through a listed extension and leave a development dependency's out", async () => {
    // write a package requiring the leaf in development only
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await writeApplication(files);
    const directory = join(root, "solo");
    await writePackage(
        directory,
        {
            name: "@fixture/solo",
            exports: { ".": "./src/index.ts" },
            devDependencies: { "@fixture/leaf": "2026.9.0" },
        },
        { id: "package-01a10b00-0000-7000-8000-000000000007" },
        {
            "index.ts": 'export * from "./marker.ts";\n',
            "marker.ts":
                '/** The marker the leaf rewrites. */\nexport const marker = "unchanged";\n',
        },
        files,
    );
    await link(root, "solo", "leaf", files);
    await writeLockfile(root, ["leaf", "middle", "other", "application", "solo"], files);
    const outputs = { bun: { kind: "module", runtime: "bun" } } as const;
    const extensions = async () =>
        (
            await loadExtensions(
                directory,
                await readPackageJson(directory, files),
                await DependencyTree.read({}, files),
                files,
            )
        ).map((entry) => entry.directory);
    // build without the development dependency's extension
    const developed = await extensions();
    await using store = await LocalPackageStore.temporary(environment);
    await using compiler = await Compiler.start(directory, { store });
    await using untouched = await compiler.build({ directory, dependencies: {}, outputs });

    // list the leaf's extension, then build again through it
    await listExtensions(directory, ["@fixture/leaf"], files);
    const listed = await extensions();
    await using transformed = await compiler.build({ directory, dependencies: {}, outputs });
    expect([
        developed,
        await readEntry(untouched, "."),
        listed,
        await readEntry(transformed, "."),
    ]).toEqual([
        [],
        { marker: "unchanged" },
        [await files.realPath(join(root, "leaf"))],
        { marker: "transformed" },
    ]);
});

test("load a listed package's extension without walking its dependencies, and build a listed member as a unit", async () => {
    // write a tool requiring the leaf and declaring an extension, which the site requires in development
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await writeApplication(files);
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
        files,
    );
    await link(root, "tool", "leaf", files);
    await writePackage(
        join(root, "site"),
        {
            name: "@fixture/site",
            exports: { ".": "./src/index.ts" },
            devDependencies: { "@fixture/other": "2026.9.0", "@fixture/tool": "2026.9.0" },
        },
        { id: "package-01a10b00-0000-7000-8000-000000000006" },
        { "index.ts": "export {};\n" },
        files,
    );
    await link(root, "site", "other", files);
    await link(root, "site", "tool", files);
    await writeLockfile(root, ["leaf", "middle", "other", "application", "tool", "site"], files);
    expect((await environment.process.spawn(["git", "init", "--quiet", root]).output()).code).toBe(
        0,
    );
    const directory = join(root, "site");
    const extensions = async () =>
        await loadExtensions(
            directory,
            await readPackageJson(directory, files),
            await DependencyTree.read({}, files),
            files,
        );
    await using store = await LocalPackageStore.temporary(environment);
    await using compilers = await CompilerPool.open(store);
    const units = async () =>
        (await Workspace.open(directory, compilers)).units.map((unit) => unit.name);
    // build the development dependencies as no units
    const developed = await units();

    // refuse a listed package declaring no extension
    await listExtensions(directory, ["@fixture/other"], files);
    expect(await refusal(extensions())).toEqual([
        "UNPROCESSABLE_CONTENT",
        "EXTENSION_INVALID",
        "@fixture/site lists @fixture/other, but @fixture/other declares no build extension",
    ]);

    // refuse a lockfile member whose directory is missing
    await writeLockfile(
        root,
        ["leaf", "middle", "other", "application", "tool", "site", "gone"],
        files,
    );
    expect(await refusal(units())).toEqual([
        "NOT_FOUND",
        "MEMBER_UNKNOWN",
        `the workspace member @fixture/gone is missing at ${join(root, "gone")}`,
    ]);
    await writeLockfile(root, ["leaf", "middle", "other", "application", "tool", "site"], files);

    // refuse a listed extension that is not installed
    await listExtensions(directory, ["@fixture/absent"], files);
    expect(await refusal(units())).toEqual([
        "UNPROCESSABLE_CONTENT",
        "DEPENDENCY_UNRESOLVED",
        "missing dependency of @fixture/site: @fixture/absent",
    ]);

    // load the tool's extension, not the leaf's it requires, and build the tool's closure first
    await listExtensions(directory, ["@fixture/tool"], files);
    const loaded = await extensions();
    const listed = await units();

    // build no unit of a package the lockfile does not list as a member
    await writeLockfile(root, ["leaf", "middle", "other", "application", "site"], files);
    const released = await units();
    expect({
        developed,
        loaded: loaded.map((entry) => entry.directory),
        listed,
        released,
    }).toEqual({
        developed: ["@fixture/site"],
        loaded: [await files.realPath(join(root, "tool"))],
        listed: ["@fixture/leaf", "@fixture/tool", "@fixture/site"],
        released: ["@fixture/site"],
    });
});

test("refuse what a module an extension generates imports until the package declares it", async () => {
    // let the leaf's listed extension, a development dependency, emit a generated entry importing the leaf
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await writeApplication(files);
    const directory = join(root, "application");
    await files.writeTextFile(join(root, "leaf", "src", "build.ts"), BARREL_EXTENSION);
    await files.writeTextFile(
        join(root, "leaf", "src", "index.ts"),
        'export const exported = "exported";\n',
    );
    await declare(directory, "devDependencies", "@fixture/leaf", files);
    await listExtensions(directory, ["@fixture/leaf"], files);
    await link(root, "application", "leaf", files);
    // refuse the generated import as an authored one
    const request = {
        directory,
        dependencies: {},
        outputs: { bun: { kind: "module", runtime: "bun" } },
    } as const;
    await using refusing = await LocalPackageStore.temporary(environment);
    await using compiler = await Compiler.start(directory, { store: refusing });
    expect(await refusal(compiler.build(request))).toEqual([
        "UNPROCESSABLE_CONTENT",
        "DEPENDENCY_UNRESOLVED",
        "undeclared runtime dependency: @fixture/leaf, imported by \0virtual:@fixture/leaf/barrel",
    ]);

    // build the application on the leaf's build once it declares the leaf at runtime
    await declare(directory, "dependencies", "@fixture/leaf", files);
    await files.writeTextFile(join(root, ".gitignore"), "node_modules/\n");
    expect((await environment.process.spawn(["git", "init", "--quiet", root]).output()).code).toBe(
        0,
    );
    await using store = await LocalPackageStore.temporary(environment);
    await using compilers = await CompilerPool.open(store);
    const workspace = await Workspace.open(directory, compilers);
    await using build = await workspace.build({ outputs: request.outputs });

    // run the generated entry with the leaf's build placed beside it, as a machine places it
    const leaf = build.manifest.outputs["bun"]?.dependencies["@fixture/leaf"];
    if (leaf?.kind !== "destack") {
        throw new TypeError("the application's output records no leaf build");
    }
    const destination = join(root, "deployed");
    await build.reader.writeDirectory(destination, files);
    const tree = await DependencyTree.materialize(store, [leaf], join(root, "builds"));
    await tree.link(join(destination, "node_modules"), files);
    const barrel = present(
        build.manifest.outputs["bun"]?.exports["barrel"]?.path,
        "the barrel entry",
    );
    expect({ ...(await import(join(destination, barrel))) }).toEqual({ exported: "exported" });
});
