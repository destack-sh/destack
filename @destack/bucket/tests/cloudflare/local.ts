import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { modulePlugin } from "@destack/package/vite";
import { ModuleTransformer } from "@destack/package/transform";
import { testHost } from "@destack/test/host";
import { rolldown } from "rolldown";
import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";

/** Compile the shared assertions once per test process. */
let compilation: Promise<string> | undefined;

/** Run the shared bucket assertions inside their Workers runtime. */
export async function runR2(name: string): Promise<void> {
    compilation ??= rolldown({
        input: fileURLToPath(new URL("./scenario.ts", import.meta.url)),
        platform: "browser",
        resolve: { conditionNames: ["workerd", "worker", "browser"] },
        external: [/^node:/u, /^cloudflare:/u],
        plugins: [modulePlugin(new ModuleTransformer(testHost().files))],
    })
        .then(async (bundle) => {
            await using _bundle = bundle;

            return await bundle.generate({ format: "esm" });
        })
        .then(({ output: [chunk] }) => chunk.code);

    // give each scenario an independent R2 binding
    const worker = new Miniflare({
        modules: true,
        script: await compilation,
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
        compatibilityFlags: ["nodejs_compat"],
        r2Buckets: ["BUCKET"],
    });
    try {
        const response = await worker.dispatchFetch(`https://storage.test/${name}`);
        const result = await response.text();
        if (response.status !== 200 || result !== "ok") {
            throw new Error(result);
        }
    } finally {
        await worker.dispose();
    }
}
