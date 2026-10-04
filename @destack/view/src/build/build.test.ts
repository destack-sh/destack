// @vitest-environment node
import { expect, test } from "@destack/test";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { PackageBuilder, readDependencies } from "@destack/build";
import { Package } from "@destack/package";
import type { Compilation, Plugin, PluginOption } from "@destack/package/build";
import type { DeclarationDescription } from "@destack/package/inspect";
import { viewExtension } from "./build.ts";
import { viewPlugins } from "./plugin.ts";

/** The notebook package, whose view module declares its notebooks view. */
const NOTEBOOK = fileURLToPath(new URL("../../tests/fixture/notebook/", import.meta.url));

/** The longest a browser build of the notebook package takes: about 13 s measured under load. */
const NOTEBOOK_BUILD_MILLISECONDS = 60_000;

/** The path of the regular trash icon, which the notebooks view draws. */
const TRASH_PATH =
    "M216,48H176V40a24,24,0,0,0-24-24H104A24,24,0,0,0,80,40v8H40a8,8,0,0,0,0,16h8V208a16,16,0,0,0,16,16H192a16,16,0,0,0,16-16V64h8a8,8,0,0,0,0-16ZM96,40a8,8,0,0,1,8-8h48a8,8,0,0,1,8,8v8H96Zm96,168H64V64H192ZM112,104v64a8,8,0,0,1-16,0V104a8,8,0,0,1,16,0Zm48,0v64a8,8,0,0,1-16,0V104a8,8,0,0,1,16,0Z";

/** The path of the regular acorn icon, which no view draws. */
const ACORN_PATH =
    "M232,104a56.06,56.06,0,0,0-56-56H136a24,24,0,0,1,24-24,8,8,0,0,0,0-16,40,40,0,0,0-40,40H80a56.06,56.06,0,0,0-56,56,16,16,0,0,0,8,13.83V128c0,35.53,33.12,62.12,59.74,83.49C103.66,221.07,120,234.18,120,240a8,8,0,0,0,16,0c0-5.82,16.34-18.93,28.26-28.51C190.88,190.12,224,163.53,224,128V117.83A16,16,0,0,0,232,104ZM80,64h96a40.06,40.06,0,0,1,40,40H40A40,40,0,0,1,80,64Zm74.25,135c-10.62,8.52-20,16-26.25,23.37-6.25-7.32-15.63-14.85-26.25-23.37C77.8,179.79,48,155.86,48,128v-8H208v8C208,155.86,178.2,179.79,154.25,199Z";

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

/** Read the code of each JavaScript chunk of a build output. */
async function readChunks(directory: string): Promise<string[]> {
    const files = await readdir(directory);

    return await Promise.all(
        files
            .filter((file) => file.endsWith(".js"))
            .map(async (file) => await readFile(join(directory, file), "utf8")),
    );
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
        // build the notebook package's browser output
        await using builder = await PackageBuilder.start(NOTEBOOK);
        await using build = await builder.build({
            dependencies: await readDependencies(NOTEBOOK),
            outputs: {
                browser: { kind: "module", runtime: "browser", bundle: true },
            },
        });

        // describe the view by its generated chunk, exported under its entrypoint
        const output = build.manifest.outputs["browser"];
        if (output === undefined) {
            throw new TypeError("missing browser output");
        }
        const entrypoint = output.views["notebooks"]?.entrypoint ?? "";
        expect([
            Object.keys(output.views),
            output.exports["./view/notebooks"] === entrypoint,
            /^output\/browser\/view-notebooks-[\w-]+\.js$/u.test(entrypoint),
        ]).toEqual([["notebooks"], true, true]);

        // bundle the drawn trash icon with the view's component and leave out every other icon
        const chunks = await readChunks(join(build.directory, "output", "browser"));
        const trash = chunks.filter((code) => code.includes(TRASH_PATH));
        const acorn = chunks.filter((code) => code.includes(ACORN_PATH));
        expect([trash.map((code) => code.includes("Notebooks")), acorn.length]).toEqual([
            [true],
            0,
        ]);
    },
);

test("generate the module mounting a view from the export declaring it, and describe it with its package's browser capabilities", () => {
    // compile a browser output of a package declaring one view, using the network and the camera
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
        constructor: {
            package: kind,
            symbol: { module: "src/declare/view.ts", name: "defineView" },
        },
        symbol: { package: owner, symbol: { module: "src/view.ts", name: "main" } },
        source: { file: "src/view.ts", line: 0, column: 0 },
        description: {
            permissions: [],
            presents: [{ packageId: owner.id, type: "note", priority: "default" }],
        },
    };
    const entries = new Map<string, string>();
    const compilation: Compilation = {
        package: owner,
        directory: "/package",
        exports: {},
        runtime: "browser",
        capabilities: {
            network: { connect: ["api.github.com"], reason: "fetches issues" },
            camera: { reason: "scans receipts", optional: true },
        },
        declarations: [view],
        modules: [],
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

    // describe the view with the camera alone, leaving the network to the workloads' sandbox
    const described = viewExtension.describe(compilation, {
        exports: { "./view/main": "output/browser/view-main.js" },
        declarations: () => [],
    });

    // import the declaring module and mount its view
    expect([[...entries.keys()], loaded.split("\n"), described]).toEqual([
        ["./view/main"],
        [
            'import { mount } from "@destack/view/browser";',
            'import { main as view } from "/package/src/view.ts";',
            "await mount(view);",
        ],
        {
            views: {
                main: {
                    entrypoint: "output/browser/view-main.js",
                    permissions: [],
                    capabilities: { camera: { reason: "scans receipts", optional: true } },
                    presents: [{ packageId: owner.id, type: "note", priority: "default" }],
                },
            },
        },
    ]);
});

test("refuse Solid server functions before their bodies can enter a browser output", () => {
    // transform a module declaring a server function through the framework plugin
    const [framework] = flatten(viewPlugins(false));

    // report the module and the directive's offset
    expect(() =>
        invoke(framework?.transform, [
            '"use server";\nexport const note = 1;\n',
            "/package/src/note.ts",
        ]),
    ).toThrow(new TypeError("solid server functions are unsupported: /package/src/note.ts:0"));
});
