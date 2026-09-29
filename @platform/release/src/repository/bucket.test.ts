import { expect, test } from "@destack/test";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { ReleaseBucket } from "./bucket.ts";
import { RepositoryConfiguration } from "./configuration.ts";
import { SigningKey } from "../key/key.ts";
import { createRoot } from "../key/root.ts";
import { createRepository, encode } from "./repository.ts";
import { publish } from "./publish.ts";

test("publish exact bytes and reject stale replacements through conditional S3 requests", async () => {
    // receive real signed HTTP requests and enforce object revisions at the storage endpoint
    const objects = new Map<string, { bytes: Buffer; revision: string }>();
    let shouldRejectCatalog = false;
    const server = Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        async fetch(request) {
            const url = new URL(request.url);
            const path = url.pathname.startsWith("/stable/")
                ? `/rehearsal${url.pathname}`
                : url.pathname;
            const current = objects.get(path);
            if (request.method === "PUT") {
                // interrupt the catalog upload after the timestamp has already been committed
                if (shouldRejectCatalog && path.endsWith("/downloads.json")) {
                    shouldRejectCatalog = false;

                    return new Response(null, { status: 503 });
                }
                const condition = request.headers.get("if-match");
                if (
                    (condition && condition !== current?.revision) ||
                    (request.headers.get("if-none-match") === "*" && current)
                ) {
                    return new Response(null, { status: 412 });
                }
                const bytes = Buffer.from(await request.arrayBuffer());
                const revision = `"${createHash("md5").update(bytes).digest("hex")}"`;
                objects.set(path, { bytes, revision });

                return new Response(null, { headers: { etag: revision } });
            }
            if (!current) {
                return new Response(null, { status: 404 });
            }

            return new Response(request.method === "HEAD" ? null : current.bytes, {
                headers: { etag: current.revision, "content-length": String(current.bytes.length) },
            });
        },
    });
    const directory = await mkdtemp(join(tmpdir(), "destack-upload-"));
    const environment = {
        DESTACK_RELEASE_CHANNEL: "stable",
        DESTACK_RELEASE_BUCKET: "rehearsal",
        DESTACK_RELEASE_S3_URL: server.url.href,
        DESTACK_RELEASE_URL: new URL("stable/", server.url).href,
        DESTACK_RELEASE_ROOT: join(directory, "root.json"),
        CLOUDFLARE_RELEASE_ACCESS_KEY_ID: "rehearsal",
        CLOUDFLARE_RELEASE_SECRET_ACCESS_KEY: "disposable-rehearsal-secret",
    };
    const previous = new Map(Object.keys(environment).map((key) => [key, process.env[key]]));
    Object.assign(process.env, environment);
    try {
        // stream an immutable artifact, accept exact retries, and refuse conflicting bytes
        const bucket = new ReleaseBucket(new RepositoryConfiguration());
        const first = join(directory, "first");
        const second = join(directory, "second");
        await writeFile(first, "first release");
        await writeFile(second, "second release");
        await bucket.put("targets/artifact", first, "application/octet-stream", true);
        await bucket.put("targets/artifact", first, "application/octet-stream", true);
        await expect(
            bucket.put("targets/artifact", second, "application/octet-stream", true),
        ).rejects.toThrow("refusing to overwrite immutable release file: targets/artifact");
        expect(await (await bucket.get("targets/artifact")).text()).toBe("first release");

        // permit one exact replacement and reject the stale writer's previous revision
        expect(await bucket.revision("metadata/timestamp.json")).toBeUndefined();
        await bucket.put("metadata/timestamp.json", first, "application/json", false);
        const revision = await bucket.revision("metadata/timestamp.json");
        await bucket.put("metadata/timestamp.json", second, "application/json", false, revision);
        await expect(
            bucket.put("metadata/timestamp.json", first, "application/json", false, revision),
        ).rejects.toThrow("release upload failed: metadata/timestamp.json (412)");
        expect(await (await bucket.get("metadata/timestamp.json")).text()).toBe("second release");
        expect([...objects.keys()].sort()).toEqual([
            "/rehearsal/stable/metadata/timestamp.json",
            "/rehearsal/stable/targets/artifact",
        ]);

        // sign a real repository with independent disposable keys
        objects.clear();
        const roots = Array.from({ length: 3 }, () => SigningKey.generate());
        const keys = {
            targets: SigningKey.generate(),
            snapshot: SigningKey.generate(),
            timestamp: SigningKey.generate(),
        };
        const root = createRoot(
            1,
            roots.map((key) => key.public),
            {
                targets: keys.targets.public,
                snapshot: keys.snapshot.public,
                timestamp: keys.timestamp.public,
            },
            new Date(Date.now() + 365 * 86_400_000).toISOString(),
        );
        root.sign((bytes) => roots[0]!.sign(bytes));
        root.sign((bytes) => roots[1]!.sign(bytes));
        await writeFile(environment.DESTACK_RELEASE_ROOT, encode(root));
        const repository = join(directory, "repository");
        await createRepository(repository, 2, root, keys, [
            { target: "aarch64-apple-darwin", version: "2026.9.1", archive: first },
        ]);
        const catalog = JSON.stringify({ version: "2026.9.1" });
        await writeFile(join(repository, "downloads.json"), catalog);

        // resume after the public timestamp advances, then accept a completed retry unchanged
        shouldRejectCatalog = true;
        await expect(publish(repository)).rejects.toThrow(
            "release upload failed: downloads.json (503)",
        );
        await publish(repository);
        const published = new Map(objects);
        await publish(repository);
        expect(objects).toEqual(published);
        expect(await (await bucket.get("downloads.json")).text()).toBe(catalog);
        expect(
            Buffer.from(await (await bucket.get("metadata/timestamp.json")).arrayBuffer()),
        ).toEqual(await readFile(join(repository, "metadata/timestamp.json")));

        // reject older revisions and conflicting signatures at the current revision before writing
        for (const revision of [1, 2]) {
            const conflicting = join(directory, `conflicting-${revision}`);
            await createRepository(conflicting, revision, root, keys, [
                { target: "aarch64-apple-darwin", version: "2026.9.2", archive: second },
            ]);
            await expect(publish(conflicting)).rejects.toThrow(
                "metadata revision must increase or retain the exact signed timestamp",
            );
            expect(objects).toEqual(published);
        }
    } finally {
        // restore credentials before releasing the disposable endpoint and files
        for (const [key, value] of previous) {
            if (value === undefined) {
                delete process.env[key];
            } else {
                process.env[key] = value;
            }
        }
        await server.stop(true);
        await rm(directory, { recursive: true, force: true });
    }
});
