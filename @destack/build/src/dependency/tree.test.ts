import { join, relative } from "node:path";
import { Digest, present, schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";

import { NpmInstallation } from "./npm.ts";
import { testSimulation } from "../test/world.ts";
import { copy, hoisted, writeStore } from "./test/store.ts";
import { BuildTree } from "./tree.ts";
import { LocalBuildStore } from "../store/local.ts";

/** The toolchain identity the units key by. */
const CONTEXT = Digest.parse("802596258260ddeb8be4565d4ec5b9afd34a32b950022f39c64bcda3ac575316");

/** The default export of a module. */
const DefaultExport = schema.looseObject({ default: schema.unknown() });

/** Encode a value as a JSON file's bytes. */
function encode(value: unknown): Uint8Array {
    return new TextEncoder().encode(JSON.stringify(value));
}

/** Settle a tree's check of a specifier as done or refused. */
function attempt(tree: BuildTree, specifier: string) {
    return refusal(() => tree.refuseInternal(specifier));
}

/** The scope of a compile's tree. */
const COMPILE = { kind: "compile" } as const;

test("place npm units of an image beside their peers, refusing a missing required peer", async () => {
    // install a renderer
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { root, locked } = await writeStore(
        {
            renderer: {
                manifest: {
                    main: "index.js",
                    peerDependencies: { core: "1.0.0", theme: "1.0.0" },
                    peerDependenciesMeta: { theme: { optional: true } },
                },
                files: { "index.js": 'module.exports = "rendered with " + require("core");\n' },
            },
            core: {
                manifest: { main: "index.js" },
                files: { "index.js": 'module.exports = "core";\n' },
            },
            styler: {
                manifest: { main: "index.js", peerDependencies: { core: "1.0.0" } },
                files: { "index.js": 'module.exports = "styled with " + require("core");\n' },
                links: ["core"],
            },
            store: {
                manifest: { main: "index.js", dependencies: { lists: "1.0.0" } },
                files: { "index.js": 'module.exports = require("lists");\n' },
                links: ["lists"],
            },
            lists: {
                manifest: { main: "index.js", peerDependencies: { store: "1.0.0" } },
                files: { "index.js": 'module.exports = typeof require("store");\n' },
            },
        },
        files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const installation = new NpmInstallation(locked, store, CONTEXT, environment);
    const build = async (name: string) =>
        await installation.store(await installation.open(copy(root, name)));
    const [renderer, core, lists, styler] = [
        await build("renderer"),
        await build("core"),
        await build("store"),
        await build("styler"),
    ];
    const listed = await installation.store(await installation.open(copy(root, "lists")));

    // place the renderer
    const tree = await BuildTree.materialize(
        store,
        ...hoisted({ renderer, core, store: lists, lists: listed }),
        join(root, "tree"),
        COMPILE,
    );
    const exported = async (name: string) =>
        DefaultExport.parse(await import(join(present(tree.directories[name], name), "index.js")))
            .default;

    // place the styler with its core
    const peered = await BuildTree.materialize(
        store,
        ...hoisted({ styler, core }),
        join(root, "peered"),
        COMPILE,
    );
    const styled = DefaultExport.parse(
        await import(join(present(peered.directories["styler"], "styler"), "index.js")),
    ).default;
    const lacking = await refusal(
        BuildTree.materialize(store, ...hoisted({ renderer }), join(root, "lacking"), COMPILE),
    );
    expect({
        renderer: await exported("renderer"),
        store: await exported("store"),
        styled,
        lacking,
    }).toEqual({
        renderer: "rendered with core",
        store: "object",
        styled: "styled with core",
        lacking: [
            "BAD_REQUEST",
            "DEPENDENCY_INVALID",
            "peer 'core' of 'renderer' is invalid: its dependent does not provide it",
        ],
    });
});

test("place a running tree's npm units as their Bun outputs under runtime roots, listing each root", async () => {
    // install a store unit depending on a list unit, each shipping its types beside its module
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { root, locked } = await writeStore(
        {
            store: {
                manifest: { main: "index.js", dependencies: { lists: "1.0.0" } },
                files: {
                    "index.js": 'module.exports = require("lists");\n',
                    "index.d.ts": "export declare const lists: string;\n",
                },
                links: ["lists"],
            },
            lists: {
                manifest: { main: "index.js" },
                files: {
                    "index.js": 'module.exports = "lists";\n',
                    "index.d.ts": "export declare const value: string;\n",
                },
            },
        },
        files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const installation = new NpmInstallation(locked, store, CONTEXT, environment);
    const opened = await installation.open(copy(root, "store"));
    const unit = await installation.store(opened);
    const lists = await installation.store(present(opened.dependencies.get("lists"), "lists"));

    // materialize both units
    const directory = join(root, "run");
    const tree = await BuildTree.materialize(store, ...hoisted({ store: unit, lists }), directory, {
        kind: "run",
        runtime: "bun",
        closure: new Set([unit.manifest, lists.manifest]),
    });
    const placed = present(tree.directories["store"], "the store unit");

    // place the module but not its types, under roots named by the runtime, and list both roots
    expect({
        exported: DefaultExport.parse(await import(join(placed, "index.js"))).default,
        isTyped: await files.exists(join(placed, "index.d.ts")),
        roots: (await tree.roots(files)).map((each) => relative(directory, each)),
    }).toEqual({
        exported: "lists",
        isTyped: false,
        roots: [
            `${lists.manifest}.bun`,
            `${unit.manifest}+${await Digest.json([`${unit.manifest} lists ${lists.manifest}`], environment.crypto)}.bun`,
        ].toSorted(),
    });
});

test("refuse an import of an internal export from outside its workspace, and allow it within", async () => {
    // place a dependency with a public and an internal export
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const directory = await files.makeTemporaryDirectory("tree-");
    await files.writeFile(
        join(directory, "package.json"),
        encode({
            name: "@example/store",
            exports: { ".": { bun: "./index.js" }, "./server": { bun: "./server.js" } },
        }),
    );
    await files.writeFile(
        join(directory, "destack.json"),
        encode({
            $schema: "https://destack.app/schemas/2026.11.0/destack.json",
            id: "package-01996ab0-0000-7000-8000-0000000000a1",
            role: "library",
            language: "typescript",
            exports: { "./server": { internal: true } },
        }),
    );

    // read the tree from outside the dependency's workspace, and from within it
    const directories = { "@example/store": directory };
    const outside = await BuildTree.read(directories, files);
    const within = await BuildTree.read(directories, files, ["@example/store"]);
    // refuse the internal export from outside alone
    expect({
        public: await attempt(outside, "@example/store"),
        outside: await attempt(outside, "@example/store/server"),
        within: await attempt(within, "@example/store/server"),
    }).toEqual({
        public: "done",
        outside: [
            "BAD_REQUEST",
            "DEPENDENCY_INVALID",
            "dependency '@example/store/server' is invalid: it is internal to the platform",
        ],
        within: "done",
    });
});
