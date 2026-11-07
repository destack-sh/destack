import { join, relative } from "node:path";
import { Digest, present, schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";

import { NpmInstallation } from "./npm.ts";
import { testSimulation } from "../test/world.ts";
import { copy, writeStore } from "./test/store.ts";
import { DependencyTree } from "./tree.ts";
import { LocalBuildStore } from "../store/local.ts";

/** The toolchain identity the units key by. */
const CONTEXT = Digest.parse("802596258260ddeb8be4565d4ec5b9afd34a32b950022f39c64bcda3ac575316");

/** The default export of a module. */
const DefaultExport = schema.looseObject({ default: schema.unknown() });

test("place npm units with the peers the packages beside them or their install provide, refusing a required peer left out and leaving an optional one", async () => {
    // install a renderer whose peers are a core package and an optional theme, a styler whose install links its core peer, and a list depending on a store whose peer it is
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
        await installation.store(await installation.open(copy(root, name)), "bun");
    const [renderer, core, lists, styler] = [
        await build("renderer"),
        await build("core"),
        await build("store"),
        await build("styler"),
    ];

    // place the renderer beside its core, and the store, whose list requires it as its peer
    const tree = await DependencyTree.materialize(
        store,
        [renderer, core, lists],
        join(root, "tree"),
    );
    const exported = async (name: string) =>
        DefaultExport.parse(await import(join(present(tree.directories[name], name), "index.js")))
            .default;

    // render with the provided core and no theme, read the store through its list, style with the core its install linked, and refuse a renderer without its core
    const alone = await DependencyTree.materialize(store, [styler], join(root, "alone"));
    const styled = DefaultExport.parse(
        await import(join(present(alone.directories["styler"], "styler"), "index.js")),
    ).default;
    const lacking = await refusal(
        DependencyTree.materialize(store, [renderer], join(root, "lacking")),
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
            "UNPROCESSABLE_CONTENT",
            "DEPENDENCY_UNRESOLVED",
            "renderer requires the peer core, which its dependent does not provide",
        ],
    });
});

test("place a running tree's npm units as their Bun outputs alone, under runtime roots, listing every root it reaches", async () => {
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
    const unit = await installation.store(await installation.open(copy(root, "store")), "bun");

    // materialize it as a running tree on Bun, over the closure of both units
    const recorded = (await store.get(unit.manifest)).outputs["bun"]?.dependencies["lists"];
    const lists = present(recorded?.kind === "npm" ? recorded : undefined, "the lists unit");
    const directory = join(root, "run");
    const tree = await DependencyTree.materialize(store, [unit], directory, {
        run: { runtime: "bun", closure: new Set([unit.manifest, lists.manifest]) },
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
        roots: [`${lists.manifest}.bun`, `${unit.manifest}.bun`].toSorted(),
    });
});

test("list the roots a layered build links, through the links inside each package a root places", async () => {
    // lay out a schema tree placing @acme/schema, which links a validator tree beside it, as a store's trees do
    await using simulation = testSimulation();
    const { files } = simulation.environment("test");
    const root = await files.makeTemporaryDirectory("linked-roots-");
    const schemaTree = join(root, "trees", "schema.bun");
    const validator = join(root, "trees", "validator.bun");
    const placed = join(schemaTree, "node_modules", "@acme", "schema");
    await files.mkdir(join(placed, "node_modules"), { recursive: true });
    await files.mkdir(join(validator, "node_modules", "validator"), { recursive: true });
    await files.symlink(
        join(validator, "node_modules", "validator"),
        join(placed, "node_modules", "validator"),
    );

    // link the schema package alone from a layered build
    const layered = join(root, "layered");
    await files.mkdir(join(layered, "node_modules", "@acme"), { recursive: true });
    await files.symlink(placed, join(layered, "node_modules", "@acme", "schema"));

    // reach both trees
    const linked = await DependencyTree.linkedRoots(layered, files);
    const real = await files.realPath(root);
    expect(linked.map((each) => relative(real, each))).toEqual([
        join("trees", "schema.bun"),
        join("trees", "validator.bun"),
    ]);
});
