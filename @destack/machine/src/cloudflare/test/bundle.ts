import { fileURLToPath } from "node:url";
import { modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import { testHost } from "@destack/test/host";
import { rolldown } from "rolldown";

/** Bundle a module for workerd with its declarations' module metadata, as one ES module. */
export async function bundleWorker(module: URL): Promise<string> {
    // bundle the module for the Workers runtime, leaving its built-in modules to it
    const bundle = await rolldown({
        input: fileURLToPath(module),
        platform: "browser",
        resolve: { conditionNames: ["workerd", "worker", "browser"] },
        external: [/^node:/u, /^cloudflare:/u],
        plugins: [modulePlugin(new ModuleTransformer(testHost().files))],
    });
    const { output } = await bundle.generate({ format: "esm" });
    const [chunk] = output;

    return chunk.code;
}
