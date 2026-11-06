import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Miniflare } from "miniflare";
import { join } from "node:path";

test("serve the release feed and its root files through the actual Worker, and reject other paths and public writes", async () => {
    // compile the deployed entrypoint for the real local Workers runtime
    const { files, process } = testHost();
    const directory = await files.makeTemporaryDirectory("destack-download-");
    onTestFinished(() => files.remove(directory, { recursive: true, force: true }));
    const entry = join(import.meta.dirname, "worker.ts");
    const words = [
        "build",
        entry,
        "--target=browser",
        "--external=node:*",
        `--outdir=${directory}`,
    ];
    const build = process.spawn([process.executable, ...words], { stdout: "ignore" });
    expect(await build.exited).toBe(0);
    const runtime = new Miniflare({
        modules: true,
        modulesRoot: directory,
        scriptPath: join(directory, "worker.js"),
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
        compatibilityFlags: ["nodejs_compat"],
        r2Buckets: ["RELEASES"],
    });

    try {
        // seed the feed with bootstrap content and immutable artifact bytes
        const stable = await runtime.getR2Bucket("RELEASES");
        await stable.put("stable/install", "stable installer", {
            httpMetadata: { contentType: "text/plain" },
        });
        const path = `stable/targets/${"a".repeat(64)}.destack.tar.gz`;
        await stable.put(path, "complete artifact");

        // serve the bootstrap files at the root, and refuse any other feed's path
        const bootstrap = await runtime.dispatchFetch("https://download.destack.sh/install");
        expect([
            bootstrap.status,
            await bootstrap.text(),
            bootstrap.headers.get("cache-control"),
        ]).toEqual([200, "stable installer", "no-store"]);
        const other = await runtime.dispatchFetch("https://download.destack.sh/nightly/install");
        expect([other.status, await other.text()]).toEqual([404, "not found"]);

        // answer the catalog only once the feed publishes it
        const absent = await runtime.dispatchFetch("https://download.destack.sh/downloads.json");
        await stable.put("stable/downloads.json", "catalog");
        const catalog = await runtime.dispatchFetch("https://download.destack.sh/downloads.json");
        expect([absent.status, catalog.status, await catalog.text()]).toEqual([
            404,
            200,
            "catalog",
        ]);

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
        expect(await (await stable.get(path))?.text()).toBe("complete artifact");
    } finally {
        await runtime.dispose();
    }
});
