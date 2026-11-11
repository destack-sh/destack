import { present } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { testSimulation } from "@destack/test/world";
import { createHash } from "node:crypto";
import { join } from "node:path";
import { R2ReleaseBucket } from "./bucket.ts";
import { RepositoryConfiguration } from "./configuration.ts";
import { publish } from "./publish.ts";
import { SignedRepository, SigningKey, TrustedRoot } from "@destack/update/publish";
import { COMMIT } from "@destack/update/test";

test("publish exact bytes at the bucket's root, and reject stale replacements through conditional S3 requests", async () => {
    // keep the repository's files in a directory of the test's own
    const host = testHost();
    const directory = await host.files.makeTemporaryDirectory("destack-upload-");
    onTestFinished(() => host.files.remove(directory, { recursive: true, force: true }));

    // run the release and its storage in a world at the host's time, which TUF checks expiry on
    const simulation = testSimulation(host.clock.now(), {
        process: {
            arguments: [],
            variables: {
                DESTACK_STAGE: "production",
                DESTACK_RELEASE_BUCKET: "rehearsal",
                DESTACK_RELEASE_S3_URL: "http://127.0.0.1:9000/",
                DESTACK_RELEASE_ORIGIN: "http://127.0.0.1:9000/",
                CLOUDFLARE_RELEASE_ACCESS_KEY_ID: "rehearsal",
                CLOUDFLARE_RELEASE_SECRET_ACCESS_KEY: "disposable-rehearsal-secret",
            },
            workingDirectory: directory,
            id: 1,
            userId: host.process.userId,
            executable: host.process.executable,
            script: host.process.script,
            runtime: host.process.runtime,
        },
    });

    // receive signed HTTP requests and enforce object revisions at the storage endpoint
    const objects = new Map<string, { bytes: Buffer; revision: string }>();
    let shouldRejectCatalog = false;
    const server = simulation.environment("storage").server.http({
        hostname: "127.0.0.1",
        port: 9000,
        async fetch(request) {
            const url = new URL(request.url);
            // read the public origin's paths as the bucket's keys, as its custom domain serves them
            const path = url.pathname.startsWith("/rehearsal/")
                ? url.pathname
                : `/rehearsal${url.pathname}`;
            const current = objects.get(path);
            if (request.method === "PUT") {
                // interrupt the catalog upload after the timestamp has already been committed
                if (shouldRejectCatalog && path.endsWith("/downloads.json")) {
                    shouldRejectCatalog = false;

                    return new Response(null, { status: 503 });
                }
                const condition = request.headers.get("if-match");
                if (
                    (condition !== null && condition !== current?.revision) ||
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

            return new Response(request.method === "HEAD" ? null : new Uint8Array(current.bytes), {
                headers: { etag: current.revision, "content-length": String(current.bytes.length) },
            });
        },
    });
    onTestFinished(() => server.stop(true));
    const environment = simulation.environment("release");
    const { files } = environment;

    // accept exact retries of an immutable artifact and refuse conflicting bytes
    const bucket = new R2ReleaseBucket(environment);
    const first = join(directory, "first");
    const second = join(directory, "second");
    await files.writeTextFile(first, "first release");
    await files.writeTextFile(second, "second release");
    await bucket.put("targets/artifact", first, "application/octet-stream", true);
    await bucket.put("targets/artifact", first, "application/octet-stream", true);
    await expect(
        bucket.put("targets/artifact", second, "application/octet-stream", true),
    ).rejects.toThrow("refusing to overwrite immutable release file: 'targets/artifact'");
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
    expect([...objects.keys()].toSorted()).toEqual([
        "/rehearsal/metadata/timestamp.json",
        "/rehearsal/targets/artifact",
    ]);

    // sign a real repository with independent disposable keys
    objects.clear();
    const roots = await Promise.all(
        Array.from({ length: 3 }, () => SigningKey.generate("ed25519", environment.crypto)),
    );
    const keys = {
        targets: await SigningKey.generate("ed25519", environment.crypto),
        snapshot: await SigningKey.generate("ed25519", environment.crypto),
        timestamp: await SigningKey.generate("ed25519", environment.crypto),
    };
    const root = TrustedRoot.create(
        1,
        roots.map((key) => key.public),
        {
            targets: keys.targets.public,
            snapshot: keys.snapshot.public,
            timestamp: keys.timestamp.public,
        },
        new Date(environment.clock.now() + 365 * 86_400_000).toISOString(),
        environment,
    );
    await present(roots[0], "the first root key").sign(root);
    await present(roots[1], "the second root key").sign(root);
    const configuration = new RepositoryConfiguration(environment, { root });
    const repository = join(directory, "repository");
    await SignedRepository.create(
        repository,
        2,
        root,
        keys,
        [
            {
                kind: "installer",
                target: "aarch64-apple-darwin",
                version: "2026.9.1",
                commit: COMMIT,
                archive: first,
                format: "tar.gz",
            },
        ],
        environment,
    );
    const catalog = JSON.stringify({ version: "2026.9.1" });
    await files.writeTextFile(join(repository, "downloads.json"), catalog);

    // resume after the public timestamp advances and accept a completed retry unchanged
    shouldRejectCatalog = true;
    await expect(publish(repository, configuration, environment)).rejects.toThrow(
        "release upload failed: downloads.json (503)",
    );
    await publish(repository, configuration, environment);
    const published = new Map(objects);
    await publish(repository, configuration, environment);
    expect(objects).toEqual(published);
    expect(await (await bucket.get("downloads.json")).text()).toBe(catalog);

    // keep each public path at the bucket's root, which the download domain serves as its own
    expect(published.has("/rehearsal/downloads.json")).toBe(true);
    expect(Buffer.from(await (await bucket.get("metadata/timestamp.json")).arrayBuffer())).toEqual(
        Buffer.from(await files.readFile(join(repository, "metadata/timestamp.json"))),
    );

    // reject older revisions and conflicting signatures at the current revision before writing
    for (const metadataRevision of [1, 2]) {
        const conflicting = join(directory, `conflicting-${metadataRevision}`);
        await SignedRepository.create(
            conflicting,
            metadataRevision,
            root,
            keys,
            [
                {
                    kind: "installer",
                    target: "aarch64-apple-darwin",
                    version: "2026.9.2",
                    commit: COMMIT,
                    archive: second,
                    format: "tar.gz",
                },
            ],
            environment,
        );
        await expect(publish(conflicting, configuration, environment)).rejects.toThrow(
            "metadata revision must increase or retain the exact signed timestamp",
        );
        expect(objects).toEqual(published);
    }
});
