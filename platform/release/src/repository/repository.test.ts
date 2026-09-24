import { expect, test } from "@destack/test";
import { Metadata, MetadataKind } from "@tufjs/models";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { SigningKey } from "../key/key.ts";
import { createRoot } from "../key/root.ts";
import { createRepository, readRepository, renewMetadata } from "./repository.ts";

test("renew freshness while preserving exact targets authorization", async () => {
    // prepare independent roles and a quorum using disposable keys
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

    // publish a complete repository and renew without the targets private key
    const directory = await mkdtemp(join(tmpdir(), "destack-repository-"));
    try {
        const archive = join(directory, "archive");
        await writeFile(archive, "distribution");
        await createRepository(directory, 1, root, keys, [
            {
                target: "aarch64-apple-darwin",
                version: "2026.9.1",
                archive,
            },
        ]);
        const original = await readFile(join(directory, "metadata/1.targets.json"));
        const renewal = { snapshot: keys.snapshot, timestamp: keys.timestamp };
        await renewMetadata(directory, 2, root, renewal, original);

        // preserve signatures, version and bytes while advancing freshness versions
        expect(await readFile(join(directory, "metadata/1.targets.json"))).toEqual(original);
        const snapshot = Metadata.fromJSON(
            MetadataKind.Snapshot,
            JSON.parse(await readFile(join(directory, "metadata/2.snapshot.json"), "utf8")),
        );
        const timestamp = Metadata.fromJSON(
            MetadataKind.Timestamp,
            JSON.parse(await readFile(join(directory, "metadata/timestamp.json"), "utf8")),
        );
        root.verifyDelegate("snapshot", snapshot);
        root.verifyDelegate("timestamp", timestamp);
        expect(snapshot.signed.version).toBe(2);
        expect(snapshot.signed.meta["targets.json"]!.version).toBe(1);
        expect(timestamp.signed.snapshotMeta.version).toBe(2);
        snapshot.signed.meta["targets.json"]!.verify(original);

        // authenticate the complete outgoing graph before allowing publication
        const verified = await readRepository(directory, root);
        expect(verified.timestamp.toJSON()).toEqual(timestamp.toJSON());
        expect(verified.snapshot.toJSON()).toEqual(snapshot.toJSON());
        expect(verified.targets.toJSON()).toEqual(JSON.parse(original.toString("utf8")));

        // reject a different authorization key even when its metadata is well formed
        const changed = JSON.parse(original.toString());
        changed.signed.version = 3;
        await expect(
            renewMetadata(directory, 3, root, renewal, Buffer.from(JSON.stringify(changed))),
        ).rejects.toThrow("targets was signed by 0/1 keys");
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
