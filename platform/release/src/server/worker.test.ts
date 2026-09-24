import { expect, test } from "@destack/test";
import { Miniflare } from "miniflare";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

test("serve isolated release feeds through the actual Worker and reject public writes", async () => {
    // compile the deployed entrypoint for the real local Workers runtime
    const directory = await mkdtemp(join(tmpdir(), "destack-download-"));
    const result = await Bun.build({
        entrypoints: [join(import.meta.dirname, "worker.ts")],
        outdir: directory,
        target: "browser",
        external: ["node:*"],
    });
    expect(result.success).toBe(true);
    const runtime = new Miniflare({
        modules: true,
        modulesRoot: directory,
        scriptPath: join(directory, "worker.js"),
        compatibilityDate: "2026-07-30",
        compatibilityFlags: ["nodejs_compat"],
        r2Buckets: ["STABLE_RELEASES", "NIGHTLY_RELEASES"],
    });

    try {
        // seed separate buckets with distinct bootstrap content and immutable artifact bytes
        const stable = await runtime.getR2Bucket("STABLE_RELEASES");
        const nightly = await runtime.getR2Bucket("NIGHTLY_RELEASES");
        await stable.put("stable/install", "stable installer", {
            httpMetadata: { contentType: "text/plain" },
        });
        await nightly.put("nightly/install", "nightly installer");
        const path = `stable/targets/${"a".repeat(64)}.destack.tar.gz`;
        await stable.put(path, "complete artifact");

        // retain the public bootstrap alias while selecting nightly only through its explicit URL
        const bootstrap = await runtime.dispatchFetch("https://download.destack.sh/install");
        expect([
            bootstrap.status,
            await bootstrap.text(),
            bootstrap.headers.get("cache-control"),
        ]).toEqual([200, "stable installer", "no-store"]);
        const preview = await runtime.dispatchFetch("https://download.destack.sh/nightly/install");
        expect([preview.status, await preview.text()]).toEqual([200, "nightly installer"]);

        // stream complete artifacts and expose matching metadata without sending a HEAD body
        const artifact = await runtime.dispatchFetch(
            `https://download.destack.sh/${path}?verify=1`,
        );
        expect([
            artifact.status,
            await artifact.text(),
            artifact.headers.get("cache-control"),
        ]).toEqual([200, "complete artifact", "public, max-age=31536000, immutable"]);
        const head = await runtime.dispatchFetch(`https://download.destack.sh/${path}`, {
            method: "HEAD",
        });
        expect([
            head.status,
            await head.text(),
            head.headers.get("content-length"),
            head.headers.get("etag"),
        ]).toEqual([200, "", "17", artifact.headers.get("etag")]);

        // deny arbitrary storage reads and every unauthenticated artifact mutation
        const missing = await runtime.dispatchFetch(
            "https://download.destack.sh/stable/private.json",
        );
        expect([missing.status, await missing.text()]).toEqual([404, "not found"]);
        const write = await runtime.dispatchFetch(`https://download.destack.sh/${path}`, {
            method: "PUT",
            body: "replacement",
        });
        expect([write.status, await write.text(), write.headers.get("allow")]).toEqual([
            405,
            "method not allowed",
            "GET, HEAD",
        ]);
        expect(await (await stable.get(path))!.text()).toBe("complete artifact");
    } finally {
        await runtime.dispose();
        await rm(directory, { recursive: true, force: true });
    }
});
