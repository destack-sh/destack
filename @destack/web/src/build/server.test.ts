import { testSimulation } from "@destack/test/world";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Window } from "happy-dom";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { DevelopmentServer } from "./server.ts";

/** The real host, whose files and network the development server runs on. */
const HOST = testHost();

/** The workspace's installed dependencies, which the application's modules resolve through. */
const INSTALLED = fileURLToPath(new URL("../../../../node_modules", import.meta.url));

/** The installed view package, which the authored application renders with. */
const VIEW = fileURLToPath(new URL("../../node_modules/@destack/view", import.meta.url));

/** This package's directory. */
const PACKAGE = fileURLToPath(new URL("../..", import.meta.url));

/** The identifier of the dependency's package. */
const DEPENDENCY = "package-01a10a00-0000-7000-8000-000000000001";

/** The identifier of the application's package. */
const APPLICATION = "package-01a10a00-0000-7000-8000-000000000002";

/** The identifier of the authored application's package. */
const AUTHORED = "package-01a10a00-0000-7000-8000-000000000003";

/** The modules of an application with authored entries, whose button counts its clicks. */
const AUTHORED_MODULES: Readonly<Record<string, string>> = {
    "src/app.tsx": [
        'import { createSignal } from "solid-js";',
        "",
        "export default function App() {",
        "    const [count, setCount] = createSignal(0);",
        "",
        "    return (",
        '        <button type="button" onClick={() => setCount(count() + 1)}>',
        "            {count()}",
        "        </button>",
        "    );",
        "}",
    ].join("\n"),
    "src/document.tsx": [
        'import type { ParentProps } from "solid-js";',
        'import { HydrationScript } from "@destack/web/render";',
        "",
        "export default function Document(properties: ParentProps) {",
        "    return (",
        '        <html lang="en">',
        "            <head>",
        "                <HydrationScript />",
        "            </head>",
        "            <body>{properties.children}</body>",
        "        </html>",
        "    );",
        "}",
    ].join("\n"),
    "src/entry-server.tsx": [
        'import { renderToStream } from "@destack/web/render";',
        'import App from "./app.tsx";',
        'import Document from "./document.tsx";',
        "",
        "export function render() {",
        "    return renderToStream(() => (",
        "        <Document>",
        "            <App />",
        "        </Document>",
        "    ));",
        "}",
    ].join("\n"),
    "src/entry-client.tsx": [
        'import { hydrate } from "@destack/web/render";',
        'import App from "./app.tsx";',
        'import Document from "./document.tsx";',
        "",
        "hydrate(",
        "    () => (",
        "        <Document>",
        "            <App />",
        "        </Document>",
        "    ),",
        "    document,",
        ");",
    ].join("\n"),
};

/** The definition of a package for the browser and Bun. */
function definition(id: string, stamps: Readonly<Record<string, { module: number }>>): string {
    const schema = "https://destack.app/schemas/2026.9.0/destack.json";

    return JSON.stringify({
        $schema: schema,
        id,
        language: "typescript",
        runtimes: ["browser", "bun"],
        stamps,
    });
}

/** Write an application that calls a local dependency into a fresh directory. */
async function writeApplication(): Promise<{
    root: string;
    application: string;
    dependency: string;
}> {
    // place the dependency beside the application, linked into its installation
    const root = await HOST.files.realPath(
        await HOST.files.makeTemporaryDirectory("destack-web-server-"),
    );
    const application = join(root, "application");
    const dependency = join(root, "dependency");
    await HOST.files.mkdir(join(application, "src"), { recursive: true });
    await HOST.files.mkdir(join(application, "node_modules", "@fixture"), { recursive: true });
    await HOST.files.mkdir(join(dependency, "src"), { recursive: true });
    await HOST.files.symlink(
        dependency,
        join(application, "node_modules", "@fixture", "dependency"),
    );
    await HOST.files.symlink(INSTALLED, join(root, "node_modules"));
    await HOST.files.writeTextFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 2, workspaces: {}, packages: {} }),
    );

    // write the dependency, labelling text by whether the build stamped its module
    const version = { version: "2026.9.0", type: "module" };
    await HOST.files.writeTextFile(
        join(dependency, "package.json"),
        JSON.stringify({
            name: "@fixture/dependency",
            ...version,
            exports: { ".": "./src/index.ts" },
        }),
    );
    await HOST.files.writeTextFile(join(dependency, "destack.json"), definition(DEPENDENCY, {}));
    await HOST.files.writeTextFile(
        join(dependency, "src", "index.ts"),
        'export function label(text: string, module?: unknown): string {\n    return `${module === undefined ? "unstamped" : "stamped"} ${text}`;\n}\n',
    );

    // write the application calling it
    await HOST.files.writeTextFile(
        join(application, "package.json"),
        JSON.stringify({
            name: "@fixture/application",
            ...version,
            dependencies: { "@fixture/dependency": "2026.9.0" },
        }),
    );
    await HOST.files.writeTextFile(join(application, "destack.json"), definition(APPLICATION, {}));
    await HOST.files.writeTextFile(
        join(application, "src", "mark.ts"),
        'export function mark(text: string, module?: unknown): string {\n    return `${module === undefined ? "unstamped" : "stamped"} ${text}`;\n}\n',
    );
    await HOST.files.writeTextFile(
        join(application, "src", "own.ts"),
        'import { mark } from "./mark.ts";\n\nexport default mark("note");\n',
    );
    await HOST.files.writeTextFile(
        join(application, "src", "app.ts"),
        'import { label } from "@fixture/dependency";\n\nexport default label("note");\n',
    );

    return { root, application, dependency };
}

/** Write an application rendering with view and web through authored entries. */
async function writeAuthoredApplication(): Promise<{ root: string; application: string }> {
    // link view, web and their Solid into the application's installation
    const root = await HOST.files.realPath(
        await HOST.files.makeTemporaryDirectory("destack-web-entry-"),
    );
    const application = join(root, "application");
    await HOST.files.mkdir(join(application, "src"), { recursive: true });
    await HOST.files.mkdir(join(application, "node_modules", "@destack"), { recursive: true });
    await HOST.files.symlink(VIEW, join(application, "node_modules", "@destack", "view"));
    await HOST.files.symlink(PACKAGE, join(application, "node_modules", "@destack", "web"));
    await HOST.files.symlink(
        join(VIEW, "node_modules", "solid-js"),
        join(application, "node_modules", "solid-js"),
    );
    await HOST.files.symlink(join(PACKAGE, "..", "..", "bun.lock"), join(root, "bun.lock"));

    // write the application and its modules
    await HOST.files.writeTextFile(
        join(application, "package.json"),
        JSON.stringify({
            name: "@fixture/authored",
            version: "2026.9.0",
            type: "module",
            dependencies: {
                "@destack/view": "2026.9.0",
                "@destack/web": "2026.9.0",
                "solid-js": "2.0.0-rc.8",
            },
        }),
    );
    await HOST.files.writeTextFile(join(application, "destack.json"), definition(AUTHORED, {}));
    for (const [path, text] of Object.entries(AUTHORED_MODULES)) {
        await HOST.files.writeTextFile(join(application, path), `${text}\n`);
    }

    return { root, application };
}

/** Load one application module through the server's current Vite server, reading its label. */
async function load(server: DevelopmentServer, path = "/src/app.ts"): Promise<unknown> {
    const module = await server.vite.ssrLoadModule(path);

    return module["default"];
}

test("restart the server with fresh package definitions once a dependency's definition changes", async () => {
    // serve the application and load it with the dependency, which watches the dependency's definition
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const { root, application, dependency } = await writeApplication();
    try {
        await using server = await DevelopmentServer.start({
            environment: world,
            directory: application,
            application: { kind: "web", app: "src/app.ts", ssr: false, publicDirectory: false },
            server: { port: 0 },
        });
        const before = server.vite;
        const loaded = await load(server);

        // wait for the watcher to follow the dependency's definition
        await expect
            .poll(() => server.vite.watcher.getWatched()[dependency], {
                timeout: 500,
                interval: 10,
            })
            .toEqual(["destack.json"]);

        // stamp the dependency's label with the calling module
        await HOST.files.writeTextFile(
            join(dependency, "destack.json"),
            definition(DEPENDENCY, { label: { module: 1 } }),
        );

        // stamp the call in the server that replaces the first once it reads the new definition
        await expect.poll(() => server.vite, { timeout: 1500, interval: 10 }).not.toBe(before);
        expect([loaded, await load(server)]).toEqual(["unstamped note", "stamped note"]);
    } finally {
        await HOST.files.remove(root, { recursive: true });
    }
});

test("restart the server with the application's own definition once it changes", async () => {
    // serve the application and load its module calling its own label
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const { root, application } = await writeApplication();
    try {
        await using server = await DevelopmentServer.start({
            environment: world,
            directory: application,
            application: { kind: "web", app: "src/app.ts", ssr: false, publicDirectory: false },
            server: { port: 0 },
        });
        const before = server.vite;
        const loaded = await load(server, "/src/own.ts");

        // wait for the watcher to follow the application's definition before stamping its label
        await expect
            .poll(() => server.vite.watcher.getWatched()[application]?.includes("destack.json"), {
                timeout: 500,
                interval: 10,
            })
            .toBe(true);
        await HOST.files.writeTextFile(
            join(application, "destack.json"),
            definition(APPLICATION, { mark: { module: 1 } }),
        );

        // stamp the call in the server that replaces the first
        await expect.poll(() => server.vite, { timeout: 1500, interval: 10 }).not.toBe(before);
        expect([loaded, await load(server, "/src/own.ts")]).toEqual([
            "unstamped note",
            "stamped note",
        ]);
    } finally {
        await HOST.files.remove(root, { recursive: true });
    }
});

test("load an authored client entry in served pages, which hydrates them", async () => {
    // serve the application through its authored entries
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const { root, application } = await writeAuthoredApplication();
    try {
        await using server = await DevelopmentServer.start({
            environment: world,
            directory: application,
            application: {
                kind: "web",
                app: "src/app.tsx",
                ssr: { runtime: "bun" },
                entryServer: "src/entry-server.tsx",
                entryClient: "src/entry-client.tsx",
                publicDirectory: false,
            },
            server: { port: 0 },
        });
        const address = server.vite.httpServer?.address();
        const port = typeof address === "object" && address !== null ? address.port : undefined;
        const url = `http://127.0.0.1:${port}/`;

        // load the client entry after the Vite client, as a generated entry loads
        const page = await (
            await HOST.network.fetch(new Request(url, { headers: { accept: "text/html" } }))
        ).text();
        const scripts = [...page.matchAll(/<script [^>]*src="[^"]*"[^>]*><\/script>/gu)];
        const entry = await HOST.network.fetch(new Request(new URL("/src/entry-client.tsx", url)));
        expect([scripts.map(([script]) => script), entry.status]).toEqual([
            [
                '<script type="module" src="/@vite/client"></script>',
                '<script type="module" src="/src/entry-client.tsx" async></script>',
            ],
            200,
        ]);

        // count a click made before the entry hydrates the page, which replays it
        const window = new Window({
            url,
            settings: {
                enableJavaScriptEvaluation: true,
                suppressInsecureJavaScriptEnvironmentWarning: true,
            },
        });
        try {
            window.document.write(page);
            window.document.querySelector("button")?.click();
            await expect
                .poll(() => window.document.querySelector("button")?.textContent, {
                    timeout: 1500,
                    interval: 10,
                })
                .toBe("1");
        } finally {
            await window.happyDOM.close();
        }
    } finally {
        await HOST.files.remove(root, { recursive: true });
    }
});

test("serve the site files an application publishes as builds write them", async () => {
    // serve an application publishing its crawler rules and model guide
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const { root, application } = await writeApplication();
    try {
        await using server = await DevelopmentServer.start({
            environment: world,
            directory: application,
            application: {
                kind: "web",
                app: "src/app.ts",
                ssr: false,
                publicDirectory: false,
                site: "https://example.test",
                metadata: {
                    robots: { rules: [{ userAgent: "*", allow: ["/"] }] },
                    llms: { title: "Example", summary: "A site.", sections: [] },
                },
            },
            server: { port: 0 },
        });
        const address = server.vite.httpServer?.address();
        const port = typeof address === "object" && address !== null ? address.port : undefined;

        // read each file with its media type
        const read = async (path: string) => {
            const response = await HOST.network.fetch(
                new Request(`http://127.0.0.1:${port}${path}`),
            );

            return [response.headers.get("content-type"), await response.text()];
        };

        expect([await read("/robots.txt"), await read("/llms.txt?fresh")]).toEqual([
            ["text/plain; charset=utf-8", "User-agent: *\nAllow: /\n"],
            ["text/plain; charset=utf-8", "# Example\n\n> A site.\n"],
        ]);
    } finally {
        await HOST.files.remove(root, { recursive: true });
    }
});
