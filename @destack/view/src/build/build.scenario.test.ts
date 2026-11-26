// @vitest-environment node
import { expect, refusal, test } from "@destack/test";
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

    // describe the view by its generated chunk, exported under its entrypoint, and the development build's frame
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
        output.frame !== undefined,
        output.exports["./view/notebooks"]?.path === entrypoint,
        entrypoint === "output/browser/view-notebooks.js",
        workers.toSorted(),
    ]).toEqual([["notebooks"], true, true, true, ["worker"]]);
});

test("generate a view's mount with its theme, actions and capabilities", () => {
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
        ...(keybinding === undefined ? {} : { keybinding }),
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
        entry: (entrypoint, module) => entries.set(entrypoint, module),
    };
    if (viewExtension.compile === undefined || viewExtension.describe === undefined) {
        throw new TypeError("expected the view build to compile and describe");
    }
    const plugins = flatten(viewExtension.compile(compilation));

    // load the entry's module through the view's plugin
    const mount = plugins.find((plugin) => plugin.name === "@destack/view");
    const module = entries.get("./view/main");
    const loaded = invoke(mount?.load, [`\0${module}`]);
    const device = invoke(mount?.load, ["\0virtual:@destack/view/device"]);
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
            'import "virtual:destack/baseline";',
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
            'import { BrowserEnvironment, serveDevice } from "@destack/view/device";',
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
