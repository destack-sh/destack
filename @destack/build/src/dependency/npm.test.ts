import { join } from "node:path";
import { Digest, present, schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";

import { NpmInstallation } from "./npm.ts";
import { testSimulation } from "../test/world.ts";
import { copy, moduleFailure, writeStore } from "./test/store.ts";
import { LocalBuildStore } from "../store/local.ts";

/** The toolchain identity the units key by, as one call's workspace reads it. */
const CONTEXT = Digest.parse("802596258260ddeb8be4565d4ec5b9afd34a32b950022f39c64bcda3ac575316");

/** The exports of a module. */
const Exports = schema.record(schema.string(), schema.unknown());

/** Call an exported function, failing for a value of another kind. */
function call(value: unknown, ...parameters: unknown[]): unknown {
    if (typeof value !== "function") {
        throw new TypeError("the module exports no such function");
    }

    return Reflect.apply(value, undefined, parameters);
}

/** Place a unit's output for a runtime in a node_modules directory, its modules read as ESM. */
async function place(
    store: LocalBuildStore,
    manifest: Digest,
    name: string,
    runtime: "bun" | "workerd",
    directory: string,
): Promise<void> {
    // write the output's files as the package, marking a converted package's modules as ESM
    const { files } = store.environment;
    const reader = await store.contents(manifest);
    const prefix = `${runtime}/`;
    for (const file of await reader.distributed()) {
        if (file.path.startsWith(prefix)) {
            const destination = join(
                directory,
                "node_modules",
                name,
                file.path.slice(prefix.length),
            );
            await files.mkdir(join(destination, ".."), { recursive: true });
            await files.writeFile(destination, await reader.load(file.path));
        }
    }

    // write a converted package's package.json, keeping a published one
    const output = present(reader.manifest.outputs[runtime], `the ${runtime} output of ${name}`);
    const exports = Object.fromEntries(
        Object.entries(output.exports).map(([subpath, module]) => [
            subpath,
            `./${module.path.slice(prefix.length)}`,
        ]),
    );
    const packageJson = join(directory, "node_modules", name, "package.json");
    if (!(await files.exists(packageJson))) {
        await files.writeTextFile(packageJson, JSON.stringify({ name, type: "module", exports }));
    }
}

/** List the runtimes a stored unit's build has outputs for. */
async function runtimesOf(store: LocalBuildStore, manifest: Digest): Promise<string[]> {
    return Object.keys((await store.contents(manifest)).manifest.outputs);
}

test("build an npm unit with an output per runtime", async () => {
    // install a package requiring another
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { root, locked } = await writeStore(
        {
            greeting: {
                manifest: { main: "index.js", dependencies: { word: "1.0.0" } },
                files: { "index.js": 'module.exports = require("word") + ", world";\n' },
                links: ["word"],
            },
            word: {
                manifest: { main: "index.js" },
                files: { "index.js": 'module.exports = "hello";\n' },
            },
        },
        files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const installation = new NpmInstallation(locked, store, CONTEXT, environment);
    const unit = await installation.open(copy(root, "greeting"));
    const stored = await installation.store(unit);
    const word = await installation.store(
        present(unit.dependencies.get("word"), "the unit of word"),
    );
    const reopened = new NpmInstallation(locked, store, CONTEXT, environment);
    const again = await reopened.store(await reopened.open(copy(root, "greeting")));

    // place and run each runtime's output
    const run = async (runtime: "bun" | "workerd") => {
        const placed = join(root, `placed-${runtime}`);
        await place(store, stored.manifest, "greeting", runtime, placed);
        await place(store, word.manifest, "word", runtime, placed);

        return schema
            .looseObject({ default: schema.string() })
            .parse(await import(join(placed, "node_modules", "greeting", "index.js"))).default;
    };
    const reader = await store.contents(stored.manifest);

    // compare the build
    expect({
        package: reader.manifest.package,
        outputs: Object.keys(reader.manifest.outputs),
        requires: reader.manifest.requires,
        imports: reader.manifest.outputs["browser"]?.exports["."]?.imports,
        greeting: [await run("bun"), await run("workerd")],
        stored: { ...stored, manifest: "" },
        isReused: again.manifest === stored.manifest,
    }).toEqual({
        package: { name: "greeting", version: "1.0.0", integrity: "sha512-greeting" },
        outputs: ["browser", "bun", "workerd"],
        requires: { word: "1.0.0" },
        imports: ["word"],
        greeting: ["hello, world", "hello, world"],
        stored: {
            kind: "npm",
            package: { name: "greeting", version: "1.0.0", integrity: "sha512-greeting" },
            manifest: "",
        },
        isReused: true,
    });
});

test("convert a package for workerd by its conditions and browser field, failing a missing host module when run", async () => {
    // install a parser whose signal reader requires the constants module lazily, once replaced and once not
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const index =
        'exports.parse = (text) => text.toUpperCase();\nexports.parseFile = async () => require("constants").SIGINT;\n';
    const { root, locked } = await writeStore(
        {
            replaced: {
                manifest: { main: "index.js", browser: { constants: false } },
                files: { "index.js": index },
            },
            unreplaced: { manifest: { main: "index.js" }, files: { "index.js": index } },
            reader: {
                manifest: { type: "module", main: "index.js" },
                files: { "index.js": 'export { SIGINT as signal } from "constants";\n' },
            },
            detector: {
                manifest: { exports: { ".": { node: "./node.js", browser: "./browser.js" } } },
                files: {
                    "node.js": 'module.exports = require("fs").existsSync("/");\n',
                    "browser.js": "module.exports = false;\n",
                },
            },
        },
        files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const installation = new NpmInstallation(locked, store, CONTEXT, environment);
    const build = async (name: string) =>
        await installation.store(await installation.open(copy(root, name)));

    // run both parsers on workerd
    const replaced = await build("replaced");
    await place(store, replaced.manifest, "replaced", "workerd", join(root, "placed"));
    const module = Exports.parse(
        await import(join(root, "placed", "node_modules", "replaced", "index.js")),
    );
    const unreplaced = await build("unreplaced");
    await place(store, unreplaced.manifest, "unreplaced", "workerd", join(root, "placed"));
    const lazy = Exports.parse(
        await import(join(root, "placed", "node_modules", "unreplaced", "index.js")),
    );
    const reader = await build("reader");
    const detector = await build("detector");
    expect({
        parsed: call(module["parse"], "hello"),
        lazy: call(lazy["parse"], "hello"),
        reader: await runtimesOf(store, reader.manifest),
        detector: await runtimesOf(store, detector.manifest),
    }).toEqual({
        parsed: "HELLO",
        lazy: "HELLO",
        reader: ["browser", "bun", "workerd"],
        detector: ["browser", "bun", "workerd"],
    });

    // fail the unreplaced parser's signal reader once it runs, as workerd lacks the constants module
    const unavailable = new Error("host module is unavailable on workerd: constants");
    await expect(call(lazy["parseFile"], "/")).rejects.toEqual(
        Object.assign(unavailable, { code: "MODULE_NOT_FOUND" }),
    );
});

test("convert a CommonJS module with a binding named like its rolldown wrapper", async () => {
    // install a module that names its required sibling as rolldown names the module itself
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { root, locked } = await writeStore(
        {
            colliding: {
                manifest: { main: "index.js" },
                files: {
                    "index.js": 'exports.answer = require("./logs/internal.js").answer;\n',
                    "logs/internal.js":
                        'const require_internal = require("../common/internal.js");\nexports.answer = require_internal.base + 1;\n',
                    "common/internal.js": "exports.base = 41;\n",
                },
            },
        },
        files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const installation = new NpmInstallation(locked, store, CONTEXT, environment);
    const unit = await installation.open(copy(root, "colliding"));
    const built = await installation.store(unit);

    // run workerd's module, exported by name
    await place(store, built.manifest, "colliding", "workerd", join(root, "placed"));
    const module = Exports.parse(
        await import(join(root, "placed", "node_modules", "colliding", "index.js")),
    );
    expect(module["answer"]).toBe(42);
});

test("convert a package publishing a tsconfig that extends an absent package", async () => {
    // install a package whose index re-exports a module, publishing a tsconfig of an absent package
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { root, locked } = await writeStore(
        {
            colors: {
                manifest: { type: "module", exports: "./dist/index.js" },
                files: {
                    "tsconfig.json": '{ "extends": "tsex/tsconfig.json" }\n',
                    "dist/index.js": 'export * from "./methods/index.js";\n',
                    "dist/methods/index.js": "export const lighten = (value) => value + 1;\n",
                },
            },
        },
        files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const installation = new NpmInstallation(locked, store, CONTEXT, environment);
    const built = await installation.store(await installation.open(copy(root, "colors")));

    // run workerd's converted modules through the re-export
    await place(store, built.manifest, "colors", "workerd", join(root, "placed"));
    const module = Exports.parse(
        await import(join(root, "placed", "node_modules", "colors", "dist", "index.js")),
    );
    expect(call(module["lighten"], 1)).toBe(2);
});

test("fail an absent optional dependency and an absent optional peer when they load", async () => {
    // install a package falling back without its optional dependency, and a loader of an optional peer nobody installs
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { root, locked } = await writeStore(
        {
            fallback: {
                manifest: { main: "index.js", optionalDependencies: { absent: "1.0.0" } },
                files: {
                    "index.js":
                        'try { module.exports = require("absent"); } catch (error) { module.exports = error.code; }\n',
                },
            },
            loader: {
                manifest: {
                    type: "module",
                    main: "index.js",
                    peerDependencies: { plugin: "1.0.0" },
                    peerDependenciesMeta: { plugin: { optional: true } },
                },
                files: {
                    "index.js":
                        'export const greet = () => "hello";\nexport const load = () => import("plugin");\n',
                },
            },
        },
        files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const placed = join(root, "placed");
    for (const name of ["fallback", "loader"]) {
        const installation = new NpmInstallation(locked, store, CONTEXT, environment);
        const stored = await installation.store(await installation.open(copy(root, name)));
        await place(store, stored.manifest, name, "workerd", placed);
    }

    // fall back when the require runs, and greet before failing to load the peer
    const fallback = schema
        .looseObject({ default: schema.string() })
        .parse(await import(join(placed, "node_modules", "fallback", "index.js")));
    const loader = Exports.parse(await import(join(placed, "node_modules", "loader", "index.js")));
    expect({ fallback: fallback.default, greeting: call(loader["greet"]) }).toEqual({
        fallback: "MODULE_NOT_FOUND",
        greeting: "hello",
    });
    const loading = join(placed, "node_modules", "loader", "index.js");
    expect(await moduleFailure(Promise.resolve(call(loader["load"])))).toEqual([
        "ERR_MODULE_NOT_FOUND",
        `Cannot find package 'plugin' imported from ${loading}`,
    ]);
});

test("require a linked required peer, leaving an optional one to dependents", async () => {
    // install a loader importing a required and an optional peer, both linked beside it
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { root, locked } = await writeStore(
        {
            host: {
                manifest: { type: "module", main: "index.js" },
                files: { "index.js": "export const host = 1;\n" },
            },
            plugin: {
                manifest: { type: "module", main: "index.js" },
                files: { "index.js": "export const plugin = 1;\n" },
            },
            loader: {
                manifest: {
                    type: "module",
                    main: "index.js",
                    peerDependencies: { host: "1.0.0", plugin: "1.0.0" },
                    peerDependenciesMeta: { plugin: { optional: true } },
                },
                files: {
                    "index.js":
                        'export { host } from "host";\nexport const load = () => import("plugin");\n',
                },
                links: ["host", "plugin"],
            },
        },
        environment.files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const installation = new NpmInstallation(locked, store, CONTEXT, environment);
    const stored = await installation.store(await installation.open(copy(root, "loader")));

    // require the required peer
    const { manifest } = await store.contents(stored.manifest);
    expect({
        requires: manifest.requires,
        peers: manifest.outputs["workerd"]?.peers,
    }).toEqual({
        requires: { host: "1.0.0" },
        peers: { host: { isOptional: false }, plugin: { isOptional: true } },
    });
});

test("build a native addon and its importer for Bun alone, and refuse a missing dependency or a cycle", async () => {
    // install a native addon, a unit importing it, a missing dependency and a cycle
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { root, locked } = await writeStore(
        {
            native: {
                manifest: { main: "index.js" },
                files: { "index.js": "module.exports = 1;\n", "addon.node": "" },
            },
            importer: {
                manifest: { main: "index.js", dependencies: { native: "1.0.0" } },
                files: { "index.js": 'module.exports = require("native");\n' },
                links: ["native"],
            },
            missing: { manifest: { dependencies: { absent: "1.0.0" } }, files: {} },
            first: {
                manifest: { dependencies: { second: "1.0.0" } },
                files: {},
                links: ["second"],
            },
            second: { manifest: { dependencies: { first: "1.0.0" } }, files: {}, links: ["first"] },
        },
        files,
    );
    await using store = await LocalBuildStore.temporary(environment);
    const installation = new NpmInstallation(locked, store, CONTEXT, environment);
    const build = async (name: string) =>
        await installation.store(await installation.open(copy(root, name)));

    // build the native addon and its importer for Bun alone
    const runtimes = async (name: string) =>
        Object.keys((await store.get((await build(name)).manifest)).outputs);
    expect([await runtimes("native"), await runtimes("importer")]).toEqual([["bun"], ["bun"]]);
    expect(await refusal(build("missing"))).toEqual([
        "BAD_REQUEST",
        "DEPENDENCY_INVALID",
        "dependency 'absent' of 'missing@1.0.0' is invalid: it is missing",
    ]);
    expect(await refusal(build("first"))).toEqual([
        "BAD_REQUEST",
        "DEPENDENCY_INVALID",
        `npm dependency '${copy(root, "first")}' is invalid: it is part of a dependency cycle`,
    ]);
});
