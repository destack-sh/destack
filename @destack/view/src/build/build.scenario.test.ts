// @vitest-environment node
import { expect, refusal, test } from "@destack/test";
import { present, Version } from "@destack/schema";
import { fileURLToPath } from "node:url";
import { Workspace, CompilerPool } from "@destack/build";
import { testStore, testSimulation } from "@destack/build/test";
import { ACTION_KIND } from "@destack/object";
import { VIEW_KIND } from "../declare/view.ts";
import { PackageVersion, type DeclarationDescription } from "@destack/package";
import type { Compilation, Plugin, PluginOption } from "@destack/package/build";
import { destackTheme } from "@destack/theme/declare";
import { viewExtension } from "./build.ts";
import { viewPlugins } from "./plugin.ts";
import { Toolchain } from "@destack/check/toolchain";

/** The notebook package, whose view module declares its notebooks view. */
const NOTEBOOK = fileURLToPath(new URL("../../tests/fixture/notebook/", import.meta.url));

/** The notes app, the notebook's views in an installable app. */
const NOTES = fileURLToPath(new URL("../../tests/fixture/notes/", import.meta.url));

/** The cipher app, which bundles the npm cipher the database builtin bundles too. */
const CIPHER = fileURLToPath(new URL("../../tests/fixture/cipher/", import.meta.url));

/** Flatten plugin options into their plugins, refusing pending ones. */
function flatten(options: readonly PluginOption[]): Plugin[] {
    const plugins: Plugin[] = [];
    for (const option of options) {
        // descend into nested options
        if (Array.isArray(option)) {
            plugins.push(...flatten(option));
        }
        // refuse plugins still loading
        else if (option instanceof Promise) {
            throw new TypeError("plugin is still loading");
        }
        // keep plugins, skipping disabled ones
        else if (option !== false && option !== null && option !== undefined) {
            plugins.push(option);
        }
    }

    return plugins;
}

/** Call a plugin hook's handler outside a bundler, without a plugin context. */
function invoke(
    hook: Plugin["load"] | Plugin["transform"],
    parameters: readonly string[],
): unknown {
    // require an object hook
    if (hook === undefined || typeof hook === "function") {
        throw new TypeError("expected an object hook");
    }

    return Reflect.apply(hook.handler, undefined, parameters);
}

test("split each view into a browser chunk, starting the device worker from its start module alone", async () => {
    // build the notebook package's browser output on its stored dependency builds, the router's JSX on view's runtime
    await using simulation = testSimulation();
    await using store = await testStore();
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const workspacePackage = await (await Workspace.open(NOTEBOOK, compilers)).package();
    const dependencies = await workspacePackage.storeDependencies({});
    const { build } = await compilers.use(NOTEBOOK, (compiler) =>
        compiler.build({
            directory: NOTEBOOK,
            dependencies,
            links: {},
            outputs: { browser: { kind: "module", runtime: "browser" } },
        }),
    );

    // describe the view by its generated chunk, exported under its entrypoint, and the development build's example page
    const output = build.manifest.outputs["browser"];
    if (output === undefined) {
        throw new TypeError("missing browser output");
    }
    const entrypoint = output.views["notebooks"]?.entrypoint ?? "";

    // list the device worker's own modules: its start alone, which loads the device module through the import map
    const workers = (await build.files())
        .map((file) => file.path)
        .filter((path) => path.startsWith("output/browser/asset/") && path.endsWith(".js"))
        .map((path) => path.slice("output/browser/asset/".length).replace(/-[\w-]{8}\.js$/u, ""));
    expect([
        Object.keys(output.views),
        output.example !== undefined,
        output.exports["./view/notebooks"]?.path === entrypoint,
        entrypoint === "output/browser/view-notebooks.js",
        workers.toSorted(),
    ]).toEqual([["notebooks"], true, true, true, ["worker"]]);
});

test("share an npm package an app's release and a builtin it links both take in", async () => {
    // release the cipher app, whose cipher the database builtin takes in too
    await using simulation = testSimulation();
    await using store = await testStore();
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const version = Version.first(simulation.environment("test").clock.now());
    const workspace = await Workspace.open(CIPHER, compilers, { version });
    const units = await (await workspace.package()).imageUnits({});

    // read the cipher packages each release imports and inlines
    const ciphers = async (name: string) => {
        const { manifest } = await store.root(present(units.get(name), name).image);
        const browser = present(manifest.outputs["browser"], `the browser output of ${name}`);
        const imports = Object.values(browser.exports).flatMap((each) => each.imports);

        return {
            imports: [...new Set(imports.filter((each) => each.startsWith("@noble/")))].toSorted(),
            inlined: Object.keys(browser.inlined).filter((each) => each.startsWith("@noble/")),
        };
    };

    // expect both to import the one shared cipher build, neither inlining a copy
    expect({
        app: await ciphers("@destack/view-cipher-fixture"),
        db: await ciphers("@destack/db"),
    }).toEqual({
        app: { imports: ["@noble/ciphers/aes.js"], inlined: [] },
        db: { imports: ["@noble/ciphers/aes.js"], inlined: [] },
    });
});

test("derive the views workload rendering an app's client views on each server output", async () => {
    // build the notes app, whose views render on the client
    await using simulation = testSimulation();
    await using store = await testStore();
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const version = Version.first(simulation.environment("test").clock.now());
    const workspace = await Workspace.open(NOTES, compilers, { version });
    const units = await (await workspace.package()).imageUnits({});
    const { image } = present(units.get("@destack/view-notes-fixture"), "the app's image");
    const { manifest } = await store.root(image);

    // describe a views workload and export its entry on Bun and workerd, and none in the browser
    const workloads = Object.fromEntries(
        Object.values(manifest.outputs).map((output) => [
            output.runtime,
            [
                Object.keys(output.workloads),
                Object.keys(output.exports).filter((subpath) => subpath.startsWith("./workload/")),
            ],
        ]),
    );
    expect(workloads).toEqual({
        browser: [[], []],
        bun: [["views"], ["./workload/views"]],
        workerd: [["views"], ["./workload/views"]],
    });
});

test("bundle an app's release image, its browser output importing builtins alone", async () => {
    // build the notes app and make its release image on its stored dependency builds
    await using simulation = testSimulation();
    await using store = await testStore();
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const version = Version.first(simulation.environment("test").clock.now());
    const workspace = await Workspace.open(NOTES, compilers, { version });
    const units = await (await workspace.package()).imageUnits({});
    const { image } = present(units.get("@destack/view-notes-fixture"), "the app's image");
    const root = await store.root(image);

    // read the npm packages each builtin's release imports
    const shared: Record<string, readonly string[]> = {};
    for (const [name, unit] of units) {
        const { manifest } = await store.root(unit.image);
        const browser = manifest.outputs["browser"];
        if (manifest.role === "builtin" && browser !== undefined) {
            const imports = Object.values(browser.exports).flatMap((each) => each.imports);
            const npm = imports.filter((each) => !each.startsWith("@destack/"));
            if (npm.length > 0) {
                shared[name] = [...new Set(npm)].toSorted();
            }
        }
    }

    // expect builtins to import only the npm units two or more of them keep, inlining the rest such as schema's zod
    expect(shared).toEqual({
        "@destack/environment": [
            "@opentelemetry/api",
            "@opentelemetry/core",
            "@opentelemetry/sdk-logs",
            "@opentelemetry/sdk-trace",
        ],
        "@destack/package": ["destack:distribution"],
        "@destack/telemetry": [
            "@opentelemetry/api",
            "@opentelemetry/core",
            "@opentelemetry/sdk-logs",
            "@opentelemetry/sdk-trace",
        ],
    });

    // read the browser modules the release root ships and the ones it maps to their sources
    const scripts = (await root.files()).filter(
        (file) => file.path.startsWith("output/browser/") && /\.[cm]?js$/u.test(file.path),
    );
    const mapped = new Set((await root.sourceMaps()).map((each) => each.generated));
    const unmapped = scripts
        .filter((file) => file.size > 0 && !mapped.has(file.path))
        .map((file) => file.path);
    // expect every module with code mapped but the export facades, which only re-export shared chunks, and the device worker's start, which its build ships unmapped
    expect(unmapped).toEqual([
        "output/browser/asset/worker-CkluHXXU.js",
        "output/browser/src/object/index.js",
        "output/browser/src/view/index.js",
    ]);

    // expect the exports to import builtins alone, every other module bundled in
    const output = present(root.manifest.outputs["browser"], "the browser output");
    const imported = Object.values(output.exports).flatMap((each) => each.imports);
    expect({
        exports: Object.keys(output.exports).toSorted(),
        imported: [...new Set(imported)].toSorted(),
    }).toEqual({
        exports: ["./device", "./object", "./objects", "./package", "./view", "./view/notebooks"],
        imported: [
            "@destack/access",
            "@destack/account/address",
            "@destack/account/browser",
            "@destack/account/object",
            "@destack/db",
            "@destack/db/browser",
            "@destack/environment/baseline",
            "@destack/environment/browser",
            "@destack/error",
            "@destack/locale",
            "@destack/object",
            "@destack/object/client",
            "@destack/object/client/browser",
            "@destack/object/device",
            "@destack/object/device/browser",
            "@destack/package",
            "@destack/package/declare",
            "@destack/package/manifest",
            "@destack/schema",
            "@destack/setting/declare",
            "@destack/setting/object",
            "@destack/sync",
            "@destack/telemetry",
            "@destack/telemetry/browser",
            "@destack/telemetry/otlp",
        ],
    });
});

test("generate a view's mount with its theme, actions and capabilities", async () => {
    // compile a browser output of a package declaring one view, a theme, two actions calling methods and one its view handles, using the network and the camera
    const owner = PackageVersion.parse({
        id: "package-01996ab0-0000-7000-8000-00000000000c",
        name: "@example/app",
        version: "2026.9.0",
    });
    const kind = PackageVersion.parse({
        id: VIEW_KIND.packageId,
        name: "@destack/view",
        version: "2026.9.0",
    });
    const objects = PackageVersion.parse({
        id: ACTION_KIND.packageId,
        name: "@destack/object",
        version: "2026.9.0",
    });
    const view: DeclarationDescription = {
        name: "main",
        kind: "view",
        package: kind,
        symbol: { package: owner, symbol: { module: "src/view.ts", name: "main" } },
        source: { file: "src/view.ts", line: 0, column: 0 },
        description: {
            presents: [{ packageId: owner.id, type: "note", priority: "default" }],
            opens: [{ packageId: owner.id, type: "note" }],
            categories: ["home"],
            audience: "person",
            packages: [],
            render: { mode: "client" },
        },
    };
    const declared = (
        declaration: "theme" | "action",
        declaring: PackageVersion,
        name: string,
        description: DeclarationDescription["description"],
    ): DeclarationDescription => ({
        ...view,
        name,
        kind: declaration,
        package: declaration === "theme" ? destackTheme.package : objects,
        symbol: { package: declaring, symbol: { module: "src/view.ts", name } },
        description,
    });
    const action = (
        name: string,
        declaring: PackageVersion,
        keybinding?: string,
        isCall = true,
    ) => ({
        package: declaring,
        name,
        title: { package: declaring.id, id: name, source: name, values: {} },
        ...(keybinding === undefined
            ? {}
            : { keybinding: { key: [keybinding], mac: [keybinding] } }),
        focus: "page",
        ...(isCall
            ? { call: { packageId: owner.id, type: "note", method: name, input: {}, inputs: [] } }
            : {}),
    });
    const ocean = { name: "ocean", package: owner };
    const entries = new Map<string, string>();
    const compilation: Compilation = {
        package: owner,
        directory: "/package",
        exports: {},
        requires: new Set(),
        packages: new Map(),
        runtime: "browser",
        sandbox: { network: { connect: ["api.github.com"] }, camera: {} },
        declarations: [
            view,
            {
                ...view,
                name: "note",
                kind: "object",
                package: objects,
                symbol: { package: owner, symbol: { module: "src/note.ts", name: "note" } },
                description: {},
            },
            declared("theme", owner, "ocean", ocean),
            declared("action", owner, "complete", action("complete", owner, "mod+enter")),
            declared("action", owner, "archive", action("archive", owner)),
            declared("action", owner, "zoom", action("zoom", owner, "mod+=", false)),
            declared("action", kind, "pin", action("pin", kind)),
        ],
        locate: (declaration) =>
            declaration.kind === "object"
                ? { file: "/package/src/note.ts", export: "note" }
                : { file: "/package/src/view.ts", export: "main" },
        definitions: async () => [],
        entry: (entrypoint, module) => entries.set(entrypoint, module),
    };
    if (viewExtension.compile === undefined || viewExtension.describe === undefined) {
        throw new TypeError("expected the view build to compile and describe");
    }
    const plugins = flatten(viewExtension.compile(compilation));

    // load the entry's module through the view's plugin
    const mount = plugins.find((plugin) => plugin.name === "@destack/view");
    const module = entries.get("./view/main");
    const loaded: unknown = await invoke(mount?.load, [`\0${module}`]);
    const device: unknown = await invoke(mount?.load, ["\0virtual:@destack/view/device"]);
    if (typeof loaded !== "string" || typeof device !== "string") {
        throw new TypeError("expected the view's and the device's module sources");
    }

    // describe the view with only the camera, leaving the network to the workloads' sandbox
    const described = viewExtension.describe(compilation, {
        exports: { "./view/main": "output/browser/view-main.js" },
        declarations: () => [],
        stylesheets: () => ["output/browser/view-main.css"],
    });

    // import the declaring module and mount its view with the package's own theme, method actions and one device worker
    const actions = [
        {
            packageId: owner.id,
            name: "complete",
            description: action("complete", owner, "mod+enter"),
        },
        { packageId: owner.id, name: "archive", description: action("archive", owner) },
    ];
    expect([[...entries.keys()], loaded.split("\n"), device.split("\n"), described]).toEqual([
        ["./view/main", "./device"],
        [
            'import "@destack/view/baseline";',
            'import "@destack/view/styles.css";',
            'import "virtual:stylex.css";',
            'import { BrowserEnvironment, mount } from "@destack/view/browser";',
            'import { main as view } from "/package/src/view.ts";',
            'import device from "virtual:@destack/view/worker?worker&url";',
            ``,
            `const worker = new URL(device, import.meta.url);`,
            `worker.searchParams.set("entry", new URL("./device.js", import.meta.url).href);`,
            `await mount(view, ${JSON.stringify({ actions, theme: ocean })}, BrowserEnvironment.system, worker);`,
            ``,
        ],
        [
            'import { BrowserEnvironment, serveDevice } from "@destack/view/device/browser";',
            'import { note as module0 } from "/package/src/note.ts";',
            ``,
            `export function serve(scope, request, loading) {`,
            `    serveDevice([module0], scope, request, ${JSON.stringify(owner)}, loading, BrowserEnvironment.system);`,
            `}`,
            ``,
        ],
        {
            views: {
                main: {
                    entrypoint: "output/browser/view-main.js",
                    stylesheets: ["output/browser/view-main.css"],
                    sandbox: { camera: {} },
                    presents: [{ packageId: owner.id, type: "note", priority: "default" }],
                    opens: [{ packageId: owner.id, type: "note" }],
                    categories: ["home"],
                    audience: "person",
                    packages: [],
                    render: { mode: "client" },
                },
            },
        },
    ]);
});

test("refuse Solid server functions in a browser output", async () => {
    // transform a module declaring a server function through the framework plugin
    const [framework] = flatten(viewPlugins(false));

    // report the module and the directive's offset
    expect(
        await refusal(() =>
            invoke(framework?.transform, [
                '"use server";\nexport const note = 1;\n',
                "/package/src/note.ts",
            ]),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "SERVER_FUNCTION_INVALID",
        "source '/package/src/note.ts' at offset 0 is invalid: solid server functions are unsupported",
    ]);
});
