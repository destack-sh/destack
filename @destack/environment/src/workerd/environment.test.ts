import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { ModuleTransformer } from "@destack/package/transform";
import { modulePlugin } from "@destack/package/vite";
import { testHost } from "@destack/test/host";
import { expect, onTestFinished, test } from "@destack/test";
import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import { rolldown } from "rolldown";

test("read a compressed response decoded by default and as stored when asked, on workerd", async () => {
    // serve a body stored compressed under its content encoding
    const text = "stored bytes ".repeat(40);
    const compressed = gzipSync(Buffer.from(text));
    const server = Bun.serve({
        port: 0,
        fetch: () => new Response(compressed, { headers: { "content-encoding": "gzip" } }),
    });
    onTestFinished(() => server.stop(true));

    // run the scenario in the Workers runtime
    const bundle = await rolldown({
        input: fileURLToPath(new URL("./test/scenario.ts", import.meta.url)),
        platform: "browser",
        resolve: { conditionNames: ["workerd", "worker", "browser"] },
        external: [/^node:/u, /^cloudflare:/u, "async_hooks", "events"],
        plugins: [modulePlugin(new ModuleTransformer(testHost()))],
    });
    const { output } = await bundle.generate({ format: "esm" });
    const worker = new Miniflare({
        modules: true,
        script: output[0].code,
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
        compatibilityFlags: ["nodejs_compat"],
    });
    onTestFinished(() => worker.dispose());

    // read the body's text length decoded, and its compressed length as stored
    const target = encodeURIComponent(`http://127.0.0.1:${server.port}/`);
    const response = await worker.dispatchFetch(`https://scenario.test/?target=${target}`);
    expect(await response.json()).toEqual({
        decoded: [text.length, "gzip"],
        stored: [compressed.length, "gzip"],
    });
});
