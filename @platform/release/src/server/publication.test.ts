import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Miniflare } from "miniflare";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { Publication } from "./publication.ts";
import { SignedRepository, SigningKey, TrustedRoot } from "@destack/update/publish";
import { COMMIT } from "@destack/update/test";

test("publish through R2 while retaining targets and making completed retries idempotent", async () => {
    // run the real local R2 engine without a public write API
    const runtime = new Miniflare({
        modules: true,
        script: "export default { fetch() { return new Response(null, { status: 404 }); } };",
        compatibilityDate: "2026-07-30",
        r2Buckets: ["RELEASES"],
    });
    const directory = await mkdtemp(join(tmpdir(), "destack-publication-"));
    try {
        // prepare a complete repository with disposable independent signing roles
        const roots = Array.from({ length: 3 }, () => SigningKey.generate());
        const keys = {
            targets: SigningKey.generate(),
            snapshot: SigningKey.generate(),
            timestamp: SigningKey.generate(),
        };
        const root = TrustedRoot.create(
            1,
            roots.map((key) => key.public),
            {
                targets: keys.targets.public,
                snapshot: keys.snapshot.public,
                timestamp: keys.timestamp.public,
            },
            new Date(Date.now() + 365 * 86_400_000).toISOString(),
        );
        for (const key of roots.slice(0, 2)) {
            root.sign((bytes) => key.sign(bytes));
        }
        const archive = join(directory, "archive");
        await writeFile(archive, "installer bytes");
        await SignedRepository.create(directory, 1, root, keys, [
            { target: "x86_64-unknown-linux-gnu", version: "2026.9.1", commit: COMMIT, archive },
        ]);

        // populate authoritative storage through the same R2 API used by the deployed Worker
        const bucket = await runtime.getR2Bucket("RELEASES");
        const metadata = join(directory, "metadata");
        const original = await readFile(join(metadata, "1.targets.json"));
        for (const name of await readdir(metadata)) {
            await bucket.put(`stable/metadata/${name}`, await readFile(join(metadata, name)));
        }
        const publication = new Publication(bucket, "stable/", root);
        await SignedRepository.renew(directory, 2, root, keys, original);
        const snapshot = await readFile(join(metadata, "2.snapshot.json"));
        const timestamp = await readFile(join(metadata, "timestamp.json"));

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
        await SignedRepository.renew(directory, 3, root, keys, original);
        const laterSnapshot = await readFile(join(metadata, "3.snapshot.json"));
        const laterTimestamp = await readFile(join(metadata, "timestamp.json"));
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
        await rm(directory, { recursive: true, force: true });
    }
});
