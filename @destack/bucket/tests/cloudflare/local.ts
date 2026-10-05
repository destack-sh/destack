import { build } from "esbuild";
import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";

/** Compile the shared assertions once per test process. */
let compilation: Promise<string> | undefined;

/** Run the shared bucket assertions inside their Workers runtime. */
export async function runR2(name: string): Promise<void> {
    compilation ??= build({
        entryPoints: [fileURLToPath(new URL("./scenario.ts", import.meta.url))],
        bundle: true,
        write: false,
        format: "esm",
        platform: "neutral",
        external: ["node:*"],
    }).then(({ outputFiles: [script] }) => {
        // require the single bundled script
        if (script === undefined) {
            throw new Error("expected one bundled scenario script");
        }

        return script.text;
    });

    // give each scenario an independent R2 binding
    const worker = new Miniflare({
        modules: true,
        script: await compilation,
        compatibilityDate: "2026-07-30",
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
