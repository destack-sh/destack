import { expect, test } from "@destack/test";
import { Metadata, MetadataKind, Snapshot, Timestamp } from "@tufjs/models";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { createRoot } from "../key/root.ts";
import { SigningKey } from "../key/key.ts";
import { createRepository, encode, renewMetadata } from "../repository/repository.ts";
import { Renewal } from "./renewal.ts";

test("renew signed freshness without permitting target replacement or rollback", async () => {
    // generate independent authorization and a complete disposable repository
    const directory = await mkdtemp(join(tmpdir(), "destack-renewal-"));
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
    for (const key of roots.slice(0, 2)) {
        root.sign((bytes) => key.sign(bytes));
    }

    try {
        // retain exact current targets and timestamp as read from authoritative storage
        const archive = join(directory, "archive");
        await writeFile(archive, "release contents");
        await createRepository(directory, 1, root, keys, [
            { target: "x86_64-unknown-linux-gnu", version: "2026.9.1", archive },
        ]);
        const targets = await readFile(join(directory, "metadata/1.targets.json"));
        const previous = Metadata.fromJSON(
            MetadataKind.Timestamp,
            JSON.parse(await readFile(join(directory, "metadata/timestamp.json"), "utf8")),
        );
        await renewMetadata(directory, 2, root, keys, targets);
        const snapshot = await readFile(join(directory, "metadata/2.snapshot.json"));
        const timestamp = await readFile(join(directory, "metadata/timestamp.json"));

        // accept the complete signed renewal while rejecting replay of its revision
        expect(new Renewal(snapshot, timestamp, targets, previous, root)).toEqual({
            revision: 2,
            snapshot,
            timestamp,
        });
        const current = Metadata.fromJSON(MetadataKind.Timestamp, JSON.parse(timestamp.toString()));
        expect(() => new Renewal(snapshot, timestamp, targets, current, root)).toThrow(
            "renewal revisions must increase",
        );

        // reject a timestamp signed by a key outside the configured root
        const unauthorized = new Metadata(Timestamp.fromJSON(current.signed.toJSON()));
        const stranger = SigningKey.generate();
        unauthorized.sign((bytes) => stranger.sign(bytes));
        expect(() => new Renewal(snapshot, encode(unauthorized), targets, previous, root)).toThrow(
            "timestamp was signed by 0/1 keys",
        );

        // permit freshness recovery from expired authoritative state without trusting expired targets
        const expired = new Metadata(
            Timestamp.fromJSON({
                ...previous.signed.toJSON(),
                expires: "2000-01-01T00:00:00Z",
            }),
        );
        expired.sign((bytes) => keys.timestamp.sign(bytes));
        expect(new Renewal(snapshot, timestamp, targets, expired, root)).toEqual({
            revision: 2,
            snapshot,
            timestamp,
        });

        // reject a legitimately signed attempt to select another targets revision
        const changed = new Metadata(
            Snapshot.fromJSON({
                ...JSON.parse(snapshot.toString()).signed,
                meta: {
                    "targets.json": {
                        version: 42,
                        length: targets.length,
                        hashes: { sha256: "a".repeat(64) },
                    },
                },
            }),
        );
        changed.sign((bytes) => keys.snapshot.sign(bytes));
        expect(() => new Renewal(encode(changed), timestamp, targets, previous, root)).toThrow(
            "renewal must preserve the current targets document",
        );

        // reject excessive validity even when the timestamp signing key authorizes it
        const extended = new Metadata(
            Timestamp.fromJSON({
                ...current.signed.toJSON(),
                expires: new Date(Date.now() + 30 * 86_400_000).toISOString(),
            }),
        );
        extended.sign((bytes) => keys.timestamp.sign(bytes));
        expect(() => new Renewal(snapshot, encode(extended), targets, previous, root)).toThrow(
            "renewal expiration exceeds its permitted lifetime",
        );
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});
