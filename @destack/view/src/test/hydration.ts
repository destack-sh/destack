import { resolve } from "node:path";
import process from "node:process";
import { loadTransforms } from "@destack/build/local";
import { modulePlugin } from "@destack/package/vite";
import { createServer } from "vite";

/** Render each component a module of the tested package exports on a server compiled as builds of server-rendered applications compile it, returning each one's HTML by export name. */
export async function renderOnServer(module: string): Promise<Record<string, string>> {
    // compile the module and Solid's server runtime for the server from the tested package, as builds do
    const root = process.cwd();
    const server = await createServer({
        root,
        configFile: false,
        logLevel: "silent",
        appType: "custom",
        server: { middlewareMode: true, hmr: false, ws: false },
        optimizeDeps: { noDiscovery: true },
        plugins: [
            modulePlugin(),
            loadTransforms({ directory: root, runtime: "bun", server: true, options: {} }),
        ],
    });
    try {
        // load the module and the runtime rendering its components
        const loaded = await server.ssrLoadModule(resolve(root, module));
        const runtime = await server.ssrLoadModule("@solidjs/web");
        const render: unknown = Reflect.get(runtime, "renderToString");
        const create: unknown = Reflect.get(runtime, "createComponent");
        if (!isRenderer(render) || !isCreator(create)) {
            throw new TypeError("the server runtime exports no renderToString or createComponent");
        }

        // render each exported component in turn, past the exports the compiler adds
        const rendered: Record<string, string> = {};
        for (const [name, component] of Object.entries(loaded)) {
            if (name.startsWith("$$")) {
                continue;
            }
            if (!isFunction(component)) {
                throw new TypeError(`${module} exports ${name}, which is no component`);
            }
            rendered[name] = await render(() => create(component, {}));
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

/** Report whether a value renders a component to HTML, as the server runtime's renderToString does. */
function isRenderer(value: unknown): value is (render: () => unknown) => Promise<string> {
    return typeof value === "function";
}

/** Report whether a value creates a component's rendering, as the server runtime's createComponent does. */
function isCreator(
    value: unknown,
): value is (component: () => unknown, properties: object) => unknown {
    return typeof value === "function";
}
