import { WORKERD_COMPATIBILITY_DATE } from "@destack/package/runtime";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Miniflare } from "miniflare";
import { join } from "node:path";
import { Publication } from "./publication.ts";
import { SignedRepository, SigningKey, TrustedRoot } from "@destack/update/publish";
import { COMMIT } from "@destack/update/test";

/** The real host the repository is signed and read on. */
const HOST = testHost();

test("publish through R2 while retaining targets and making completed retries idempotent", async () => {
    // run the real local R2 engine without a public write API
    const runtime = new Miniflare({
        modules: true,
        script: "export default { fetch() { return new Response(null, { status: 404 }); } };",
        compatibilityDate: WORKERD_COMPATIBILITY_DATE,
        r2Buckets: ["RELEASES"],
    });
    const directory = await HOST.files.makeTemporaryDirectory("destack-publication-");
    try {
        // prepare a complete repository with disposable independent signing roles
        const roots = await Promise.all(
            Array.from({ length: 3 }, () => SigningKey.generate("ed25519", HOST.crypto)),
        );
        const keys = {
            targets: await SigningKey.generate("ed25519", HOST.crypto),
            snapshot: await SigningKey.generate("ed25519", HOST.crypto),
            timestamp: await SigningKey.generate("ed25519", HOST.crypto),
        };
        const root = TrustedRoot.create(
            1,
            roots.map((key) => key.public),
            {
                targets: keys.targets.public,
                snapshot: keys.snapshot.public,
                timestamp: keys.timestamp.public,
            },
            new Date(HOST.clock.now() + 365 * 86_400_000).toISOString(),
            HOST,
        );
        for (const key of roots.slice(0, 2)) {
            await key.sign(root);
        }
        const archive = join(directory, "archive");
        await HOST.files.writeTextFile(archive, "installer bytes");
        await SignedRepository.create(
            directory,
            1,
            root,
            keys,
            [{ target: "x86_64-unknown-linux-gnu", version: "2026.9.1", commit: COMMIT, archive }],
            HOST,
        );

        // populate authoritative storage through the same R2 API used by the deployed Worker
        const bucket = await runtime.getR2Bucket("RELEASES");
        const metadata = join(directory, "metadata");
        const original = Buffer.from(await HOST.files.readFile(join(metadata, "1.targets.json")));
        for (const { name } of await HOST.files.readDirectory(metadata)) {
            await bucket.put(
                `stable/metadata/${name}`,
                Buffer.from(await HOST.files.readFile(join(metadata, name))),
            );
        }
        const publication = new Publication(bucket, "stable/", root, HOST.clock);
        await SignedRepository.renew(directory, 2, root, keys, original, HOST);
        const snapshot = Buffer.from(await HOST.files.readFile(join(metadata, "2.snapshot.json")));
        const timestamp = Buffer.from(await HOST.files.readFile(join(metadata, "timestamp.json")));

        // publish and replay the exact completed request without changing its selected timestamp
        await publication.renew(snapshot, timestamp);
        const first = present(
            await bucket.get("stable/metadata/timestamp.json"),
            "the published timestamp",
        );
        expect(Buffer.from(await first.arrayBuffer())).toEqual(timestamp);
        await publication.renew(snapshot, timestamp);
        expect((await bucket.head("stable/metadata/timestamp.json"))?.etag).toBe(first.etag);
        expect(
            Buffer.from(
                await present(
                    await bucket.get("stable/metadata/1.targets.json"),
                    "the first targets",
                ).arrayBuffer(),
            ),
        ).toEqual(original);

        // preserve a later publication when an earlier request arrives again
        await SignedRepository.renew(directory, 3, root, keys, original, HOST);
        const laterSnapshot = Buffer.from(
            await HOST.files.readFile(join(metadata, "3.snapshot.json")),
        );
        const laterTimestamp = Buffer.from(
            await HOST.files.readFile(join(metadata, "timestamp.json")),
        );
        await publication.renew(laterSnapshot, laterTimestamp);
        await expect(publication.renew(snapshot, timestamp)).rejects.toThrow(
            "renewal revisions must increase",
        );
        expect(
            Buffer.from(
                await present(
                    await bucket.get("stable/metadata/timestamp.json"),
                    "the later timestamp",
                ).arrayBuffer(),
            ),
        ).toEqual(laterTimestamp);
        expect((await bucket.list()).objects.map((object) => object.key)).toEqual([
            "stable/metadata/1.root.json",
            "stable/metadata/1.snapshot.json",
            "stable/metadata/1.targets.json",
            "stable/metadata/2.snapshot.json",
            "stable/metadata/3.snapshot.json",
            "stable/metadata/timestamp.json",
        ]);
    } finally {
        await runtime.dispose();
        await HOST.files.remove(directory, { recursive: true, force: true });
    }
});
