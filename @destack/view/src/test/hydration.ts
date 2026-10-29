import { resolve } from "node:path";
import { loadTransforms } from "@destack/build/local";
import { BrowserEnvironment } from "@destack/environment/browser";
import { testHost } from "@destack/test/host";
import { modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import { createServer } from "vite";

/** Render each component a module of the tested package exports on a server. */
export async function renderOnServer(module: string): Promise<Record<string, string>> {
    // compile the module and Solid's server runtime for the server from the tested package, as builds do
    const host = testHost();
    const root = host.process.workingDirectory;
    const server = await createServer({
        root,
        configFile: false,
        logLevel: "silent",
        appType: "custom",
        server: { middlewareMode: true, hmr: false, ws: false },
        optimizeDeps: { noDiscovery: true },
        plugins: [
            modulePlugin(new ModuleTransformer(host.files)),
            loadTransforms({
                directory: root,
                runtime: "bun",
                server: true,
                options: {},
                environment: host,
            }),
        ],
    });
    try {
        // load the module, the runtime rendering its components and the environment context they read
        const loaded = await server.ssrLoadModule(resolve(root, module));
        const runtime = await server.ssrLoadModule("@solidjs/web");
        const view = await server.ssrLoadModule("@destack/view");
        const context: unknown = Reflect.get(view, "EnvironmentContext");
        const render: unknown = Reflect.get(runtime, "renderToString");
        const create: unknown = Reflect.get(runtime, "createComponent");
        if (!isRenderer(render) || !isCreator(create) || !isFunction(context)) {
            throw new TypeError(
                "the server runtime has no renderToString, createComponent or environment context",
            );
        }

        // render each exported component in turn on a server render's environment, past the exports the compiler adds
        const environment = BrowserEnvironment.server(host);
        const rendered: Record<string, string> = {};
        for (const [name, component] of Object.entries(loaded)) {
            if (name.startsWith("$$")) {
                continue;
            }
            if (!isFunction(component)) {
                throw new TypeError(`${module} exports ${name}, which is not a component`);
            }
            rendered[name] = await render(() =>
                create(context, {
                    value: environment,
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
