import { testSimulation } from "@destack/test/world";
import type { Environment } from "@destack/environment";
import { PackageVersion } from "@destack/package";
import { present, schema } from "@destack/schema";
import { BuildWriter } from "@destack/package/manifest";
import { afterEach, expect, test, vi } from "@destack/test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build, createServer, type Plugin, type Rolldown, type ViteDevServer } from "vite";
import { styleExtension } from "./build.ts";

/** The dependency whose build keeps plum's rules. */
const DEPENDENCY = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d1",
    name: "@destack/style-dependency",
    version: "2026.9.0",
});

/** The package directory, where the fixture modules resolve `@destack/style`. */
const ROOT = fileURLToPath(new URL("../..", import.meta.url));

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
            } else if (id.endsWith("/fixture/pair.ts")) {
                return `import "/fixture/teal";\nimport "/fixture/plum";\n`;
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

            // write a module whose style stands under a constant another module defines
            else if (id.endsWith("/fixture/hover.ts")) {
                return `import * as style from "@destack/style";\nimport { media } from "../../style/media.stylex.ts";\nexport const styles = style.create({ text: { color: { default: "teal", ":hover": { default: null, [media.hover]: "plum" } } } });\n`;
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
                environment,
            }),
        ],
        build: { write: false, minify: false, rolldownOptions: { input: [entry].flat() } },
    });
}

/** The milliseconds a sheet reload waits for further transforms, as the plugin sets it. */
const SHEET_SETTLE = 150;

/** Wait for the reloads queued so far, moving the world's time past their settling. */
async function flushPending(simulation: {
    readonly sleep: (milliseconds: number) => Promise<void>;
}): Promise<void> {
    await simulation.sleep(SHEET_SETTLE);
}

/** Start a development server compiling styles as a package that uses `@destack/style` does. */
async function serve(environment: Environment<"files">): Promise<ViteDevServer> {
    const server = await createServer({
        root: ROOT,
        configFile: false,
        logLevel: "silent",
        appType: "custom",
        server: { middlewareMode: true, hmr: false, ws: false, preTransformRequests: false },
        optimizeDeps: { noDiscovery: true },
        plugins: [
            fixtures(),
            styleExtension.transform({
                directory: ROOT,
                dependencies: {},
                runtime: "browser",
                server: false,
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
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const server = await serve(environment);
    const client = server.environments.client;
    await client.transformRequest("/fixture/teal");
    await flushPending(simulation);
    await client.transformRequest("virtual:stylex.css");
    const sheet = await client.pluginContainer.load("\0virtual:stylex.css");
    const reload = vi.spyOn(client, "reloadModule").mockResolvedValue();
    await client.transformRequest("/fixture/teal");
    await flushPending(simulation);
    const unchanged = reload.mock.calls.length;

    // compile a style with a new rule
    await client.transformRequest("/fixture/plum");
    await flushPending(simulation);

    expect({ sheet, unchanged, grown: reload.mock.calls.length }).toEqual({
        sheet: "@layer base;\n\n@layer priority1 {\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
        unchanged: 0,
        grown: 1,
    });
});

test("serve the last sheet until the constants its styles stand under are compiled", async () => {
    // serve the sheet with the style alone, then with the constant
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const server = await serve(environment);
    const client = server.environments.client;
    await client.transformRequest("/fixture/hover");
    const reload = vi.spyOn(client, "reloadModule").mockResolvedValue();
    await client.transformRequest("virtual:stylex.css");
    const without = await client.pluginContainer.load("\0virtual:stylex.css");
    await client.transformRequest("/src/style/media.stylex.ts");
    await flushPending(simulation);

    expect({
        without,
        reloads: reload.mock.calls.length,
        with: await client.pluginContainer.load("\0virtual:stylex.css"),
    }).toEqual({
        without: "",
        reloads: 1,
        with: "@layer base;\n\n@layer priority1 {\n  .xy1w8w {\n    color: teal;\n  }\n}\n\n@layer priority2 {\n  @media (hover: hover) {\n    .xaomgs3.xaomgs3:hover {\n      color: plum;\n    }\n  }\n}\n",
    });
});

test("reload the sheet once for a burst of new rules, never for a transform repeating served ones", async () => {
    // compile one style and serve the sheet with its rule
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const server = await serve(environment);
    const client = server.environments.client;
    await client.transformRequest("/fixture/teal");
    await flushPending(simulation);
    await client.transformRequest("virtual:stylex.css");
    const reload = vi.spyOn(client, "reloadModule").mockResolvedValue();

    // compile the served style again, as a second environment or an edit keeping its rules does
    const teal = present(await client.moduleGraph.getModuleByUrl("/fixture/teal"), "teal module");
    client.moduleGraph.invalidateModule(teal);
    await client.transformRequest("/fixture/teal");
    await flushPending(simulation);
    const repeated = reload.mock.calls.length;

    // compile two styles with new rules in one burst
    await Promise.all(
        ["/fixture/plum", "/fixture/coral"].map((url) => client.transformRequest(url)),
    );
    await flushPending(simulation);

    expect({ repeated, burst: reload.mock.calls.length }).toEqual({ repeated: 0, burst: 1 });
});

test("serve the sheet without the StyleX plugin's timer polling its rule version", async () => {
    // record every interval a development server starts
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const intervals = vi.spyOn(globalThis, "setInterval");
    await serve(environment);
    const delays = intervals.mock.calls.map(([, delay]) => delay);
    intervals.mockRestore();

    // start no interval at StyleX's 150 ms polling delay
    expect(delays.filter((delay) => delay === 150)).toEqual([]);
});

test("append every rule to the entry's stylesheet, leaving lazy pages' stylesheets", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    expect(await stylesheets(environment, "/fixture/entry")).toEqual({
        "entry.css":
            "\n@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
        "page.css": ".page { margin: 0 }\n",
    });
});

test("append every rule to the stylesheet of a chunk several entries share", async () => {
    // read the sheets whatever the bundler names the shared chunk
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const sheets = await stylesheets(environment, ["/fixture/one", "/fixture/two"]);

    expect(Object.values(sheets).toSorted()).toEqual([
        "\n@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
        ".page { margin: 0 }\n",
    ]);
});

test("keep a build's rules for the builds depending on it when no module imports the sheet", async () => {
    // read the rules file the bundle keeps
    await using simulation = testSimulation();
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

test("keep a build's rules sorted by module path, whatever order the modules compile in", async () => {
    // read the rules file of a bundle compiling teal before plum
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const result = await bundle(environment, "/fixture/pair");
    const outputs = (Array.isArray(result) ? result : [result]).flatMap((each) =>
        "output" in each ? each.output : [],
    );
    const rules = outputs.find((each) => each.fileName === "stylex.json");
    const source = rules?.type === "asset" ? String(rules.source) : "";

    // list plum before teal, so equal sources keep equal bytes
    const modules = schema
        .array(schema.tuple([schema.string(), schema.unknown()]))
        .parse(JSON.parse(source))
        .map(([module]) => module);
    expect(modules).toEqual(["src/build/fixture/plum.ts", "src/build/fixture/teal.ts"]);
});

test("add the rules each dependency's build keeps to the stylesheet, ordered with the build's own", async () => {
    // keep a dependency's rules as its build keeps them
    await using simulation = testSimulation();
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
    await writer.finish({ destack: "2026.11.0", role: "app", package: DEPENDENCY });

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
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await bundle(environment, "/fixture/entry");
    const sheets = await stylesheets(environment, "/fixture/solo");

    expect(sheets).toEqual({
        "solo.css": "\n@layer base;\n\n@layer priority1 {\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
    });
});
