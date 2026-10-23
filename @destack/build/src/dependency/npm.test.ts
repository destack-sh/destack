import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { Digest, schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";

import { NpmInstallation } from "./npm.ts";
import { convertHere, copy, moduleFailure, writeStore } from "./test/store.ts";
import { LocalPackageStore } from "../store/local.ts";

/** The toolchain identity the units key by, as one call's workspace reads it. */
const CONTEXT = await Digest.json({ test: "toolchain" });

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
    store: LocalPackageStore,
    manifest: string,
    name: string,
    runtime: "bun" | "workerd",
    directory: string,
): Promise<void> {
    // write the output's files as the package, marking a converted package's modules as ESM
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
            await mkdir(join(destination, ".."), { recursive: true });
            await writeFile(destination, await reader.load(file.path));
        }
    }
    if (runtime === "workerd") {
        const packageJson = join(directory, "node_modules", name, "package.json");
        const declared = schema
            .looseObject({})
            .parse(JSON.parse(await Bun.file(packageJson).text()));
        await writeFile(packageJson, JSON.stringify({ ...declared, type: "module" }));
    }
}

/** List the runtimes a stored unit's build has outputs for. */
async function runtimesOf(store: LocalPackageStore, manifest: string): Promise<string[]> {
    return Object.keys((await store.contents(manifest)).manifest.outputs);
}

test("build an npm unit as installed for Bun and as ESM for workerd, on its dependency's unit", async () => {
    // install a CommonJS package requiring another
    const { root, locked } = await writeStore({
        greeting: {
            manifest: { main: "index.js", dependencies: { word: "1.0.0" } },
            files: { "index.js": 'module.exports = require("word") + ", world";\n' },
            links: ["word"],
        },
        word: {
            manifest: { main: "index.js" },
            files: { "index.js": 'module.exports = "hello";\n' },
        },
    });
    await using store = await LocalPackageStore.temporary();
    const installation = new NpmInstallation(locked, store, CONTEXT);
    const unit = await installation.open(copy(root, "greeting"));
    const stored = await installation.store(unit, convertHere);
    const reopened = new NpmInstallation(locked, store, CONTEXT);
    const again = await reopened.store(await reopened.open(copy(root, "greeting")), convertHere);

    // place each workerd output where node_modules resolves it, and run the converted module
    const reader = await store.contents(stored.manifest);
    const recorded = reader.manifest.outputs["workerd"]?.dependencies["word"];
    if (recorded?.kind !== "npm") {
        throw new TypeError("the workerd output records no npm unit of word");
    }
    const placed = join(root, "placed");
    await place(store, stored.manifest, "greeting", "workerd", placed);
    await place(store, recorded.manifest, "word", "workerd", placed);
    const module = schema
        .looseObject({ default: schema.string() })
        .parse(await import(join(placed, "node_modules", "greeting", "index.js")));

    // store Bun's copy byte for byte, record the release with its build, and reuse the stored build
    expect({
        package: reader.manifest.package,
        outputs: Object.keys(reader.manifest.outputs),
        bun: new TextDecoder().decode(await reader.load("bun/index.js")),
        greeting: module.default,
        stored: { ...stored, manifest: "" },
        isReused: again.manifest === stored.manifest,
    }).toEqual({
        package: { name: "greeting", version: "1.0.0", integrity: "sha512-greeting" },
        outputs: ["bun", "workerd"],
        bun: 'module.exports = require("word") + ", world";\n',
        greeting: "hello, world",
        stored: { kind: "npm", ...locked["greeting@1.0.0"], manifest: "" },
        isReused: true,
    });
});

test("convert for workerd what its conditions import, replacing what a `browser` field empties and failing a require of a missing host module when it runs", async () => {
    // install a parser whose signal reader requires the constants module lazily, once replaced and once not
    const index =
        'exports.parse = (text) => text.toUpperCase();\nexports.parseFile = async () => require("constants").SIGINT;\n';
    const { root, locked } = await writeStore({
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
    });
    await using store = await LocalPackageStore.temporary();
    const installation = new NpmInstallation(locked, store, CONTEXT);
    const build = async (name: string) =>
        await installation.store(await installation.open(copy(root, name)), convertHere);

    // run the replaced parser on workerd's modules, and build no workerd output of the other
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
        reader: ["bun", "workerd"],
        detector: ["bun", "workerd"],
    });

    // fail the unreplaced parser's signal reader once it runs, as workerd lacks the constants module
    const unavailable = new Error("host module is unavailable on workerd: constants");
    await expect(call(lazy["parseFile"], "/")).rejects.toEqual(
        Object.assign(unavailable, { code: "MODULE_NOT_FOUND" }),
    );
});

test("convert a package for workerd past the tsconfig it publishes, which extends a development dependency, as khroma's does", async () => {
    // install a package whose index re-exports a module, publishing a tsconfig of an absent package
    const { root, locked } = await writeStore({
        colors: {
            manifest: { type: "module", exports: "./dist/index.js" },
            files: {
                "tsconfig.json": '{ "extends": "tsex/tsconfig.json" }\n',
                "dist/index.js": 'export * from "./methods/index.js";\n',
                "dist/methods/index.js": "export const lighten = (value) => value + 1;\n",
            },
        },
    });
    await using store = await LocalPackageStore.temporary();
    const installation = new NpmInstallation(locked, store, CONTEXT);
    const built = await installation.store(
        await installation.open(copy(root, "colors")),
        convertHere,
    );

    // run workerd's converted modules through the re-export
    await place(store, built.manifest, "colors", "workerd", join(root, "placed"));
    const module = Exports.parse(
        await import(join(root, "placed", "node_modules", "colors", "dist", "index.js")),
    );
    expect(call(module["lighten"], 1)).toBe(2);
});

test("fail an absent optional dependency's require when it runs, and an absent optional peer's import once imported", async () => {
    // install a package falling back without its optional dependency, and a loader of an optional peer nobody installs
    const { root, locked } = await writeStore({
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
    });
    await using store = await LocalPackageStore.temporary();
    const placed = join(root, "placed");
    for (const name of ["fallback", "loader"]) {
        const installation = new NpmInstallation(locked, store, CONTEXT);
        const stored = await installation.store(
            await installation.open(copy(root, name)),
            convertHere,
        );
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

test("refuse an npm unit with a missing dependency or a cycle, and build Bun's copy alone of a native addon and its importers", async () => {
    // install a native addon, a unit importing it, a missing dependency and a cycle
    const { root, locked } = await writeStore({
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
        first: { manifest: { dependencies: { second: "1.0.0" } }, files: {}, links: ["second"] },
        second: { manifest: { dependencies: { first: "1.0.0" } }, files: {}, links: ["first"] },
    });
    await using store = await LocalPackageStore.temporary();
    const installation = new NpmInstallation(locked, store, CONTEXT);
    const build = async (name: string) =>
        await installation.store(await installation.open(copy(root, name)), convertHere);

    // build Bun's copy alone of the native addon and its importer, and refuse the others with their exact reasons
    expect(await runtimesOf(store, (await build("native")).manifest)).toEqual(["bun"]);
    expect(await runtimesOf(store, (await build("importer")).manifest)).toEqual(["bun"]);
    expect(await refusal(build("missing"))).toEqual([
        "UNPROCESSABLE_CONTENT",
        "DEPENDENCY_UNRESOLVED",
        "missing dependency of missing@1.0.0: absent",
    ]);
    expect(await refusal(build("first"))).toEqual([
        "UNPROCESSABLE_CONTENT",
        "DEPENDENCY_INVALID",
        `npm dependency cycle through ${copy(root, "first")}`,
    ]);
});
