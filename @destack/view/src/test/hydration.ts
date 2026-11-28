import { ActionRegistry } from "../page/action.ts";
import { resolve } from "node:path";
import { loadTransforms } from "@destack/build/local";
import { BrowserEnvironment } from "@destack/environment/browser";
import { testHost } from "@destack/test/host";
import { modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import { createServer } from "vite";

/** The module of the page's root, loaded alone to skip compiling the package's index. */
const ROOT_MODULE = resolve(import.meta.dirname, "../page/root.ts");

/** Render each component a module of the tested package exports on a server, at a page's URL. */
export async function renderOnServer(
    module: string,
    url = "https://example.test/",
): Promise<Record<string, string>> {
    // compile the module and Solid's server runtime for the server from the tested package, as builds do
    const host = testHost();
    const root = host.process.workingDirectory;
    const cache = await host.files.makeTemporaryDirectory("destack-hydration-");
    try {
        const server = await createServer({
            root,
            // give this server its own dependency cache
            cacheDir: cache,
            configFile: false,
            logLevel: "silent",
            appType: "custom",
            server: { middlewareMode: true, hmr: false, ws: false },
            optimizeDeps: { noDiscovery: true },
            plugins: [
                modulePlugin(new ModuleTransformer(host)),
                loadTransforms({
                    directory: root,
                    runtime: "bun",
                    server: true,
                    environment: host,
                }),
            ],
        });
        try {
            // load the module, the runtime rendering its components and the root they render under
            const loaded = await server.ssrLoadModule(resolve(root, module));
            const runtime = await server.ssrLoadModule("@solidjs/web");
            const view = await server.ssrLoadModule(ROOT_MODULE);
            const pageRoot: unknown = Reflect.get(view, "PageRoot");
            const render: unknown = Reflect.get(runtime, "renderToString");
            const create: unknown = Reflect.get(runtime, "createComponent");
            if (!isRenderer(render) || !isCreator(create) || !isFunction(pageRoot)) {
                throw new TypeError(
                    "the server runtime has no renderToString, createComponent or page root",
                );
            }

            // render each exported component in turn on a server render's environment, past the exports the compiler adds
            const environment = BrowserEnvironment.server(host, url);
            const rendered: Record<string, string> = {};
            for (const [name, component] of Object.entries(loaded)) {
                if (name.startsWith("$$")) {
                    continue;
                }
                if (!isFunction(component)) {
                    throw new TypeError(`'${module}' exports '${name}', which is not a component`);
                }
                rendered[name] = await render(() =>
                    create(pageRoot, {
                        environment,
                        actions: new ActionRegistry(),
                        get children() {
                            return create(component, {});
                        },
                    }),
                );
            }

            return rendered;
        } finally {
            await server.close();
        }
    } finally {
        // remove the server's cache, also after a failed start
        await host.files.remove(cache, { recursive: true });
    }
}

/** Report whether a value is a function. */
function isFunction(value: unknown): value is () => unknown {
    return typeof value === "function";
}

/** Report whether a value renders a component to HTML. */
function isRenderer(value: unknown): value is (render: () => unknown) => Promise<string> {
    return typeof value === "function";
}

/** Report whether a value creates a component's rendering. */
function isCreator(
    value: unknown,
): value is (component: () => unknown, properties: object) => unknown {
    return typeof value === "function";
}
