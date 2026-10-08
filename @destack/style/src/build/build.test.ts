import { afterEach, expect, test, vi } from "@destack/test";
import { fileURLToPath } from "node:url";
import { build, createServer, type Plugin, type Rolldown, type ViteDevServer } from "vite";
import { styleExtension } from "./build.ts";

/** The package directory, where the fixture modules resolve `@destack/style`. */
const ROOT = fileURLToPath(new URL("../..", import.meta.url));

/** The development servers to close after each test. */
const servers: ViteDevServer[] = [];

afterEach(async () => {
    await Promise.all(servers.splice(0).map((server) => server.close()));
});

/** Serve modules by their name: an entry, a lazy page with a stylesheet, and StyleX styles of one color each. */
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

/** Build an entry with styles compiled as a package that uses `@destack/style` does. */
async function bundle(entry: string): ReturnType<typeof build> {
    return await build({
        root: ROOT,
        configFile: false,
        logLevel: "silent",
        plugins: [
            fixtures(),
            styleExtension.transform({
                directory: ROOT,
                runtime: "browser",
                server: false,
                options: {},
            }),
        ],
        build: { write: false, minify: false, rolldownOptions: { input: entry } },
    });
}

/** Wait for the reloads queued in the current task. */
async function settle(): Promise<void> {
    await new Promise((resolve) => {
        setTimeout(resolve);
    });
}

/** Start a development server compiling styles as a package that uses `@destack/style` does. */
async function serve(): Promise<ViteDevServer> {
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
                runtime: "browser",
                server: false,
                options: {},
            }),
        ],
    });
    servers.push(server);

    return server;
}

test("serve the collected rules as a stylesheet module, reloading it when transforms add rules", async () => {
    // compile one style, request the sheet and read its rules, then compile the style again
    const server = await serve();
    const client = server.environments.client;
    await client.transformRequest("/fixture/teal");
    await settle();
    await client.transformRequest("virtual:stylex.css");
    const sheet = await client.pluginContainer.load("\0virtual:stylex.css");
    const reload = vi.spyOn(client, "reloadModule").mockResolvedValue();
    await client.transformRequest("/fixture/teal");
    await settle();
    const unchanged = reload.mock.calls.length;

    // compile a style with a new rule
    await client.transformRequest("/fixture/plum");
    await settle();

    expect({ sheet, unchanged, grown: reload.mock.calls.length }).toEqual({
        sheet: "@layer base;\n\n@layer priority1 {\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
        unchanged: 0,
        grown: 1,
    });
});

test("append every rule to the entry's stylesheet, leaving lazy pages' stylesheets", async () => {
    // build the entry and its lazy page
    const output = await bundle("/fixture/entry");

    // read each stylesheet by the chunk that loads it
    const results = Array.isArray(output) ? output : [output];
    const files = results.flatMap(
        (result): readonly (Rolldown.OutputChunk | Rolldown.OutputAsset)[] =>
            "output" in result ? result.output : [],
    );
    const sheets = Object.fromEntries(
        files
            .filter(
                (file): file is Rolldown.OutputAsset =>
                    file.type === "asset" && file.fileName.endsWith(".css"),
            )
            .map((file): [string, string] => [file.names.join(", "), String(file.source)]),
    );

    expect(sheets).toEqual({
        "entry.css":
            "\n@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
        "page.css": ".page { margin: 0 }\n",
    });
});

test("refuse rules in a bundle whose only stylesheets belong to lazy pages", async () => {
    await expect(bundle("/fixture/bare")).rejects.toThrow(
        "styles need a stylesheet the entry loads",
    );
});
