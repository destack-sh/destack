import { build } from "esbuild";
import { Miniflare } from "miniflare";
import { fileURLToPath } from "node:url";
import { expect, onTestFinished, test } from "@destack/test";

test("migrate, write and read a Durable Object's SQLite storage, rolling back only a failed nested transaction", async () => {
    // bundle the scenario and run it in the Workers runtime with one Durable Object
    const compiled = await build({
        entryPoints: [fileURLToPath(new URL("./test/scenario.ts", import.meta.url))],
        bundle: true,
        write: false,
        format: "esm",
        platform: "browser",
        conditions: ["workerd", "worker", "browser"],
        external: ["node:*", "cloudflare:*"],
    });
    const [output] = compiled.outputFiles;
    if (output === undefined) {
        throw new Error("the scenario bundled into no file");
    }
    const worker = new Miniflare({
        modules: true,
        script: output.text,
        compatibilityDate: "2026-07-30",
        compatibilityFlags: ["nodejs_compat"],
        durableObjects: { NOTES: { className: "Notes", useSQLite: true } },
    });
    onTestFinished(() => worker.dispose());

    // keep the written and the outer rows, without the inner one
    const response = await worker.dispatchFetch("https://notes.test/");
    expect([response.status, await response.json()]).toEqual([
        200,
        [
            { id: "a", title: "Changed" },
            { id: "b", title: "Outer" },
        ],
    ]);
});
