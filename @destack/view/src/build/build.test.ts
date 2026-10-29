// @vitest-environment node
import { expect, refusal, test } from "@destack/test";
import { fileURLToPath } from "node:url";
import { Workspace, CompilerPool } from "@destack/build";
import { testStore, testSimulation } from "@destack/build/test";
import { Package, type DeclarationDescription } from "@destack/package";
import type { Compilation, Plugin, PluginOption } from "@destack/package/build";
import { destackTheme } from "@destack/theme/declare";
import { viewExtension } from "./build.ts";
import { viewPlugins } from "./plugin.ts";

/** The notebook package, whose view module declares its notebooks view. */
const NOTEBOOK = fileURLToPath(new URL("../../tests/fixture/notebook/", import.meta.url));

/** The longest a browser build of the notebook package takes: 1.8 s alone, 7 s under load. */
const NOTEBOOK_BUILD_MILLISECONDS = 60_000;

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
            throw new TypeError("unexpected pending plugin");
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

test(
    "mount each declared view from a separate browser chunk and describe it in the output",
    { timeout: NOTEBOOK_BUILD_MILLISECONDS },
    async () => {
        // build the notebook package's browser output on its stored dependency builds, the router's JSX on view's runtime
        await using simulation = testSimulation();
        await using store = await testStore();
        await using compilers = await CompilerPool.open(store);
        const workspace = await Workspace.open(NOTEBOOK, compilers);
        const dependencies = await workspace.storeDependencies({});
        await using build = await compilers.use(NOTEBOOK, (compiler) =>
            compiler.build({
                directory: NOTEBOOK,
                dependencies,
                outputs: { browser: { kind: "module", runtime: "browser" } },
            }),
        );

        // describe the view by its generated chunk, exported under its entrypoint
        const output = build.manifest.outputs["browser"];
        if (output === undefined) {
            throw new TypeError("missing browser output");
        }
        const entrypoint = output.views["notebooks"]?.entrypoint ?? "";
        expect([
            Object.keys(output.views),
            output.exports["./view/notebooks"]?.path === entrypoint,
            entrypoint === "output/browser/view-notebooks.js",
        ]).toEqual([["notebooks"], true, true]);
    },
);

test("generate the module mounting a view with its package's theme and commands, and describe it with its package's browser capabilities", () => {
    // compile a browser output of a package declaring one view, a theme and two commands, using the network and the camera
    const owner = Package.parse({
        id: "package-01996ab0-0000-7000-8000-00000000000c",
        name: "@example/app",
        version: "2026.9.0",
    });
    const kind = Package.parse({
        id: "package-01996ab0-0000-7000-8000-00000000000d",
        name: "@destack/view",
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
            packages: [],
        },
    };
    const declared = (
        declaration: "theme" | "command",
        declaring: Package,
        name: string,
        description: DeclarationDescription["description"],
    ): DeclarationDescription => ({
        ...view,
        name,
        kind: declaration,
        package: declaration === "theme" ? destackTheme.package : view.package,
        symbol: { package: declaring, symbol: { module: "src/view.ts", name } },
        description,
    });
    const command = (name: string, keybinding?: string) => ({
        title: name,
        packageId: owner.id,
        type: "note",
        method: name,
        ...(keybinding === undefined ? {} : { keybinding }),
    });
    const ocean = { name: "ocean", package: owner };
    const entries = new Map<string, string>();
    const compilation: Compilation = {
        package: owner,
        directory: "/package",
        exports: {},
        runtime: "browser",
        sandbox: { network: { connect: ["api.github.com"] }, camera: {} },
        declarations: [
            view,
            declared("theme", owner, "ocean", ocean),
            declared("command", owner, "complete", command("complete", "mod+enter")),
            declared("command", owner, "archive", command("archive")),
            declared("command", kind, "pin", command("pin")),
        ],
        locate: () => ({ file: "/package/src/view.ts", export: "main" }),
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
    if (typeof loaded !== "string") {
        throw new TypeError("expected the view's module source");
    }

    // describe the view with only the camera, leaving the network to the workloads' sandbox
    const described = viewExtension.describe(compilation, {
        exports: { "./view/main": "output/browser/view-main.js" },
        declarations: () => [],
    });

    // import the declaring module and mount its view with the package's own theme and commands
    const commands = [
        { packageId: owner.id, name: "complete", keybinding: "mod+enter" },
        { packageId: owner.id, name: "archive" },
    ];
    expect([[...entries.keys()], loaded.split("\n"), described]).toEqual([
        ["./view/main"],
        [
            'import { BrowserEnvironment, mount } from "@destack/view/browser";',
            'import { main as view } from "/package/src/view.ts";',
            `await mount(view, ${JSON.stringify({ commands, theme: ocean })}, BrowserEnvironment.system);`,
        ],
        {
            views: {
                main: {
                    entrypoint: "output/browser/view-main.js",
                    sandbox: { camera: {} },
                    presents: [{ packageId: owner.id, type: "note", priority: "default" }],
                    packages: [],
                },
            },
        },
    ]);
});

test("refuse Solid server functions before their bodies can enter a browser output", async () => {
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
        "UNPROCESSABLE_CONTENT",
        "SERVER_FUNCTION_UNSUPPORTED",
        "solid server functions are unsupported: /package/src/note.ts:0",
    ]);
});
