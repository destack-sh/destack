import type { Environment } from "@destack/environment";
import { Package } from "@destack/package";
import { BuildWriter } from "@destack/package/manifest";
import { Simulation } from "@destack/environment/simulation";
import { afterEach, expect, test, vi } from "@destack/test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build, createServer, type Plugin, type Rolldown, type ViteDevServer } from "vite";
import { styleExtension } from "./build.ts";

/** The dependency whose build keeps plum's rules. */
const DEPENDENCY = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d1",
    name: "@destack/style-dependency",
    version: "2026.9.0",
});

/** The package directory, where the fixture modules resolve `@destack/style`. */
const ROOT = fileURLToPath(new URL("../..", import.meta.url));

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The development servers to close after each test. */
const servers: ViteDevServer[] = [];

afterEach(async () => {
    await Promise.all(servers.splice(0).map((server) => server.close()));
});

/** Serve modules by name: an entry, a lazy page with a stylesheet, and one-color StyleX styles. */
function fixtures(): Plugin {
    return {
        name: "fixtures",
        resolveId: (id) =>
            id.startsWith("/fixture/")
                ? `${ROOT}src/build${id}${id.endsWith(".css") ? "" : ".ts"}`
                : undefined,
        load: (id) => {
            // write the entry, which loads the sheet and a style and the page later, and one loading only the page
            if (id.endsWith("/fixture/entry.ts")) {
                return `import "virtual:stylex.css";\nimport "/fixture/teal";\nvoid import("/fixture/page");\n`;
            } else if (id.endsWith("/fixture/bare.ts")) {
                return `void import("/fixture/page");\n`;
            } else if (id.endsWith("/fixture/solo.ts")) {
                return `import "virtual:stylex.css";\nimport "/fixture/teal";\n`;
            }
            // write two entries sharing the module that loads the sheet and a style
            else if (id.endsWith("/fixture/one.ts") || id.endsWith("/fixture/two.ts")) {
                return `import "/fixture/shared";\nvoid import("/fixture/page");\n`;
            } else if (id.endsWith("/fixture/shared.ts")) {
                return `import "virtual:stylex.css";\nimport "/fixture/teal";\n`;
            }
            // write the lazy page, with a stylesheet of its own and a style
            else if (id.endsWith("/fixture/page.ts")) {
                return `import "/fixture/page.css";\nimport "/fixture/plum";\n`;
            } else if (id.endsWith("/fixture/page.css")) {
                return ".page { margin: 0 }\n";
            }

            // write a module whose one style sets the color its name gives
            const color = /\/fixture\/([a-z]+)\.ts$/u.exec(id)?.[1];
            if (color === undefined) {
                return undefined;
            }

            return `import * as style from "@destack/style";\nexport const styles = style.create({ text: { color: "${color}" } });\n`;
        },
    };
}

/** Build entries with styles compiled as a package that uses `@destack/style` does. */
async function bundle(
    environment: Environment<"files">,
    entry: string | readonly string[],
    dependencies: Readonly<Record<string, string>> = {},
): ReturnType<typeof build> {
    return await build({
        root: ROOT,
        configFile: false,
        logLevel: "silent",
        plugins: [
            fixtures(),
            styleExtension.transform({
                directory: ROOT,
                dependencies,
                runtime: "browser",
                server: false,
                options: {},
                environment,
            }),
        ],
        build: { write: false, minify: false, rolldownOptions: { input: [entry].flat() } },
    });
}

/** Wait for the reloads queued in the current task. */
async function settle(environment: Environment): Promise<void> {
    await environment.clock.sleep(0);
}

/** Start a development server compiling styles as a package that uses `@destack/style` does. */
async function serve(environment: Environment<"files">): Promise<ViteDevServer> {
    const server = await createServer({
        root: ROOT,
        configFile: false,
        logLevel: "silent",
        appType: "custom",
        server: { middlewareMode: true, hmr: false, ws: false },
        optimizeDeps: { noDiscovery: true },
        plugins: [
            fixtures(),
            styleExtension.transform({
                directory: ROOT,
                dependencies: {},
                runtime: "browser",
                server: false,
                options: {},
                environment,
            }),
        ],
    });
    servers.push(server);

    return server;
}

/** Build entries and read each stylesheet by the names of the chunks that load it. */
async function stylesheets(
    environment: Environment<"files">,
    entry: string | readonly string[],
    dependencies: Readonly<Record<string, string>> = {},
): Promise<Record<string, string>> {
    // build the entries
    const output = await bundle(environment, entry, dependencies);
    const results = Array.isArray(output) ? output : [output];
    const files = results.flatMap(
        (result): readonly (Rolldown.OutputChunk | Rolldown.OutputAsset)[] =>
            "output" in result ? result.output : [],
    );

    return Object.fromEntries(
        files
            .filter(
                (file): file is Rolldown.OutputAsset =>
                    file.type === "asset" && file.fileName.endsWith(".css"),
            )
            .map((file): [string, string] => [file.names.join(", "), String(file.source)]),
    );
}

test("serve the collected rules as a stylesheet module, reloading it when transforms add rules", async () => {
    // compile one style, request the sheet and read its rules, then compile the style again
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const server = await serve(environment);
    const client = server.environments.client;
    await client.transformRequest("/fixture/teal");
    await settle(environment);
    await client.transformRequest("virtual:stylex.css");
    const sheet = await client.pluginContainer.load("\0virtual:stylex.css");
    const reload = vi.spyOn(client, "reloadModule").mockResolvedValue();
    await client.transformRequest("/fixture/teal");
    await settle(environment);
    const unchanged = reload.mock.calls.length;

    // compile a style with a new rule
    await client.transformRequest("/fixture/plum");
    await settle(environment);

    expect({ sheet, unchanged, grown: reload.mock.calls.length }).toEqual({
        sheet: "@layer base;\n\n@layer priority1 {\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
        unchanged: 0,
        grown: 1,
    });
});

test("serve the sheet without the StyleX plugin's timer polling its rule version", async () => {
    // record every interval a development server starts
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const intervals = vi.spyOn(globalThis, "setInterval");
    await serve(environment);
    const delays = intervals.mock.calls.map(([, delay]) => delay);
    intervals.mockRestore();

    // start no interval at StyleX's 150 ms polling delay
    expect(delays.filter((delay) => delay === 150)).toEqual([]);
});

test("append every rule to the entry's stylesheet, leaving lazy pages' stylesheets", async () => {
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    expect(await stylesheets(environment, "/fixture/entry")).toEqual({
        "entry.css":
            "\n@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
        "page.css": ".page { margin: 0 }\n",
    });
});

test("append every rule to the stylesheet of a chunk several entries share", async () => {
    // read the sheets whatever the bundler names the shared chunk
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const sheets = await stylesheets(environment, ["/fixture/one", "/fixture/two"]);

    expect(Object.values(sheets).toSorted()).toEqual([
        "\n@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
        ".page { margin: 0 }\n",
    ]);
});

test("keep a build's rules for the builds depending on it when no module imports the sheet", async () => {
    // read the rules file the bundle keeps
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const result = await bundle(environment, "/fixture/bare");
    const outputs = (Array.isArray(result) ? result : [result]).flatMap((each) =>
        "output" in each ? each.output : [],
    );
    const rules = outputs.find((each) => each.fileName === "stylex.json");
    const source = rules?.type === "asset" ? String(rules.source) : undefined;

    expect(source).toEqual(
        '[["src/build/fixture/plum.ts",[["xjjgwud",{"ltr":".xjjgwud{color:plum}","rtl":null},3000]]]]',
    );
});

test("add the rules each dependency's build keeps to the stylesheet, ordered with the build's own", async () => {
    // keep a dependency's rules as its build keeps them
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    const { files } = environment;
    const dependency = await files.makeTemporaryDirectory("style-dependency-");
    const rules = JSON.stringify([
        ["src/plum.ts", [["xjjgwud", { ltr: ".xjjgwud{color:plum}", rtl: null }, 3000]]],
    ]);

    // write the dependency's build with its rules file
    const writer = new BuildWriter(
        {
            write: async (path, bytes) => {
                await files.mkdir(dirname(join(dependency, path)), { recursive: true });
                await files.writeFile(join(dependency, path), bytes);
            },
        },
        environment.crypto,
    );
    await writer.write("output/library/stylex.json", new TextEncoder().encode(rules));
    await writer.finish({ package: DEPENDENCY });

    // build teal with the dependency's plum
    const sheets = await stylesheets(environment, "/fixture/solo", {
        "@destack/style-dependency": dependency,
    });

    expect(sheets).toEqual({
        "solo.css":
            "\n@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
    });
});

test("keep only a build's own rules after an earlier build in the same process", async () => {
    // build a bundle using plum and teal, then one using teal alone
    await using simulation = new Simulation({ seed: 1, start: START });
    const environment = simulation.environment("test");
    await bundle(environment, "/fixture/entry");
    const sheets = await stylesheets(environment, "/fixture/solo");

    expect(sheets).toEqual({
        "solo.css": "\n@layer base;\n\n@layer priority1 {\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
    });
});
