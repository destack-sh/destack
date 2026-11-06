import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Metadata, MetadataKind, Snapshot, Timestamp } from "@tufjs/models";
import { join } from "node:path";
import { Renewal } from "./renewal.ts";
import { MetadataDocument } from "../document.ts";
import { SignedRepository, SigningKey, TrustedRoot } from "@destack/update/publish";
import { COMMIT } from "@destack/update/test";

/** The real host the repository is signed and read on. */
const HOST = testHost();

test("renew signed freshness without permitting target replacement or rollback", async () => {
    // generate independent authorization and a complete disposable repository
    const directory = await HOST.files.makeTemporaryDirectory("destack-renewal-");
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

    try {
        // retain exact current targets and timestamp as read from authoritative storage
        const archive = join(directory, "archive");
        await HOST.files.writeTextFile(archive, "release contents");
        await SignedRepository.create(
            directory,
            1,
            root,
            keys,
            [{ target: "x86_64-unknown-linux-gnu", version: "2026.9.1", commit: COMMIT, archive }],
            HOST,
        );
        const targets = Buffer.from(
            await HOST.files.readFile(join(directory, "metadata/1.targets.json")),
        );
        const previous = Metadata.fromJSON(
            MetadataKind.Timestamp,
            MetadataDocument.parse(
                await HOST.files.readTextFile(join(directory, "metadata/timestamp.json")),
            ),
        );
        await SignedRepository.renew(directory, 2, root, keys, targets, HOST);
        const snapshot = Buffer.from(
            await HOST.files.readFile(join(directory, "metadata/2.snapshot.json")),
        );
        const timestamp = Buffer.from(
            await HOST.files.readFile(join(directory, "metadata/timestamp.json")),
        );

        // accept the complete signed renewal while rejecting replay of its revision
        const now = new Date(HOST.clock.now());
        expect(new Renewal(snapshot, timestamp, targets, previous, root, now)).toEqual({
            revision: 2,
            snapshot,
            timestamp,
        });
        const current = Metadata.fromJSON(
            MetadataKind.Timestamp,
            MetadataDocument.parse(timestamp.toString()),
        );
        expect(() => new Renewal(snapshot, timestamp, targets, current, root, now)).toThrow(
            "renewal revisions must increase",
        );

        // reject a timestamp signed by a key outside the configured root
        const unauthorized = new Metadata(Timestamp.fromJSON(current.signed.toJSON()));
        const stranger = await SigningKey.generate("ed25519", HOST.crypto);
        await stranger.sign(unauthorized);
        expect(
            () =>
                new Renewal(
                    snapshot,
                    SignedRepository.encode(unauthorized),
                    targets,
                    previous,
                    root,
                    now,
                ),
        ).toThrow("timestamp was signed by 0/1 keys");

        // permit freshness recovery from expired authoritative state without trusting expired targets
        const expired = new Metadata(
            Timestamp.fromJSON({
                ...previous.signed.toJSON(),
                expires: "2000-01-01T00:00:00Z",
            }),
        );
        await keys.timestamp.sign(expired);
        expect(new Renewal(snapshot, timestamp, targets, expired, root, now)).toEqual({
            revision: 2,
            snapshot,
            timestamp,
        });

        // reject a legitimately signed attempt to select another targets revision
        const changed = new Metadata(
            Snapshot.fromJSON({
                ...Metadata.fromJSON(
                    MetadataKind.Snapshot,
                    MetadataDocument.parse(snapshot.toString()),
                ).signed.toJSON(),
                meta: {
                    "targets.json": {
                        version: 42,
                        length: targets.length,
                        hashes: { sha256: "a".repeat(64) },
                    },
                },
            }),
        );
        await keys.snapshot.sign(changed);
        expect(
            () =>
                new Renewal(
                    SignedRepository.encode(changed),
                    timestamp,
                    targets,
                    previous,
                    root,
                    now,
                ),
        ).toThrow("renewal must preserve the current targets document");

        // reject excessive validity even when the timestamp signing key authorizes it
        const extended = new Metadata(
            Timestamp.fromJSON({
                ...current.signed.toJSON(),
                expires: new Date(HOST.clock.now() + 30 * 86_400_000).toISOString(),
            }),
        );
        await keys.timestamp.sign(extended);
        expect(
            () =>
                new Renewal(
                    snapshot,
                    SignedRepository.encode(extended),
                    targets,
                    previous,
                    root,
                    now,
                ),
        ).toThrow("renewal expiration exceeds its permitted lifetime");
    } finally {
        await HOST.files.remove(directory, { recursive: true, force: true });
    }
});
