import { TestUpdate } from "../test/index.ts";
import { expect, refusal, test } from "@destack/test";
import { Metadata, MetadataKind, Targets } from "@tufjs/models";
import { join } from "node:path";
import { SigningKey } from "./key.ts";
import { TrustedRoot } from "./root.ts";
import { parseObject } from "./json.ts";
import { type Distribution, SignedRepository } from "./repository.ts";
import { Manifest } from "../manifest/manifest.ts";
import { COMMIT } from "../test/index.ts";

test("renew freshness while preserving exact targets authorization", async () => {
    // run in a world paused at the system's time, near the wall clock TUF checks expiry on
    const world = TestUpdate.world().environment("test");

    // prepare independent roles and a quorum using disposable keys
    const first = await SigningKey.generate("ed25519", world.crypto);
    const second = await SigningKey.generate("ed25519", world.crypto);
    const roots = [first, second, await SigningKey.generate("ed25519", world.crypto)];
    const keys = {
        targets: await SigningKey.generate("ed25519", world.crypto),
        snapshot: await SigningKey.generate("ed25519", world.crypto),
        timestamp: await SigningKey.generate("ed25519", world.crypto),
    };
    const root = TrustedRoot.create(
        1,
        roots.map((key) => key.public),
        {
            targets: keys.targets.public,
            snapshot: keys.snapshot.public,
            timestamp: keys.timestamp.public,
        },
        new Date(world.clock.now() + 365 * 86_400_000).toISOString(),
        world,
    );
    await first.sign(root);
    await second.sign(root);

    // publish a complete repository and renew without the targets private key
    const directory = await world.files.makeTemporaryDirectory("destack-repository-");
    try {
        const archive = join(directory, "archive");
        await world.files.writeTextFile(archive, "distribution");
        await SignedRepository.create(
            directory,
            1,
            root,
            keys,
            [
                {
                    kind: "installer",
                    target: "aarch64-apple-darwin",
                    version: "2026.9.1",
                    commit: COMMIT,
                    archive,
                    format: "tar.gz",
                },
            ],
            world,
        );
        const original = Buffer.from(
            await world.files.readFile(join(directory, "metadata/1.targets.json")),
        );
        const renewal = { snapshot: keys.snapshot, timestamp: keys.timestamp };
        await SignedRepository.renew(directory, 2, root, renewal, original, world);

        // preserve signatures, version and bytes while advancing freshness versions
        expect(
            Buffer.from(await world.files.readFile(join(directory, "metadata/1.targets.json"))),
        ).toEqual(original);
        const snapshot = Metadata.fromJSON(
            MetadataKind.Snapshot,
            parseObject(
                await world.files.readTextFile(join(directory, "metadata/2.snapshot.json")),
            ),
        );
        const timestamp = Metadata.fromJSON(
            MetadataKind.Timestamp,
            parseObject(await world.files.readTextFile(join(directory, "metadata/timestamp.json"))),
        );
        root.verifyDelegate("snapshot", snapshot);
        root.verifyDelegate("timestamp", timestamp);
        const reference = snapshot.signed.meta["targets.json"];
        if (reference === undefined) {
            throw new Error("expected a targets reference in the snapshot");
        }
        expect(snapshot.signed.version).toBe(2);
        expect(reference.version).toBe(1);
        expect(timestamp.signed.snapshotMeta.version).toBe(2);
        reference.verify(original);

        // authenticate the complete outgoing graph before allowing publication
        const verified = await SignedRepository.read(directory, root, world);
        expect(verified.timestamp.toJSON()).toEqual(timestamp.toJSON());
        expect(verified.snapshot.toJSON()).toEqual(snapshot.toJSON());
        expect(verified.targets.toJSON()).toEqual(JSON.parse(original.toString("utf8")));

        // reject a different authorization key even when its metadata is well formed
        const signed = Metadata.fromJSON(MetadataKind.Targets, parseObject(original.toString()));
        const changed = new Metadata(
            Targets.fromJSON({ ...signed.signed.toJSON(), version: 3 }),
            signed.signatures,
        );
        await expect(
            SignedRepository.renew(
                directory,
                3,
                root,
                renewal,
                SignedRepository.encode(changed),
                world,
            ),
        ).rejects.toThrow("targets was signed by 0/1 keys");
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});

test("publish a toolchain's files as blobs and no target, refusing files its manifest lacks", async () => {
    // sign a root over disposable ed25519 keys in a world paused near the wall clock
    const world = TestUpdate.world().environment("test");
    const roots = await Promise.all(
        Array.from({ length: 3 }, () => SigningKey.generate("ed25519", world.crypto)),
    );
    const keys = {
        targets: await SigningKey.generate("ed25519", world.crypto),
        snapshot: await SigningKey.generate("ed25519", world.crypto),
        timestamp: await SigningKey.generate("ed25519", world.crypto),
    };
    const root = TrustedRoot.create(
        1,
        roots.map((key) => key.public),
        {
            targets: keys.targets.public,
            snapshot: keys.snapshot.public,
            timestamp: keys.timestamp.public,
        },
        new Date(world.clock.now() + 365 * 86_400_000).toISOString(),
        world,
    );
    for (const key of roots.slice(0, 2)) {
        await key.sign(root);
    }

    // lay out a toolchain whose files lost their modes, which its manifest records
    const directory = await world.files.makeTemporaryDirectory("destack-repository-");
    const toolchain = join(directory, "toolchain");
    await world.files.mkdir(join(toolchain, "bin"), { recursive: true });
    await world.files.writeTextFile(join(toolchain, "bin", "tsc"), "compile");
    await world.files.writeTextFile(join(toolchain, "lint.js"), "lint");
    const read = await Manifest.read(toolchain, world);
    const manifest = new Manifest(
        read.files.map((file) => (file.path === "bin/tsc" ? { ...file, mode: 0o755 } : file)),
        read.links,
    );
    const published: Extract<Distribution, { kind: "toolchain" }> = {
        kind: "toolchain",
        target: "aarch64-apple-darwin",
        version: "2026.9.1",
        commit: COMMIT,
        directory: toolchain,
        manifest,
    };
    try {
        // publish the toolchain, then refuse a manifest of other files
        const targets = await SignedRepository.create(
            join(directory, "feed"),
            1,
            root,
            keys,
            [published],
            world,
        );
        const other = new Manifest(
            manifest.files.map((file) => ({ ...file, size: file.size + 1 })),
            [],
        );
        const refused = await refusal(
            SignedRepository.create(
                join(directory, "refused"),
                1,
                root,
                keys,
                [{ ...published, manifest: other }],
                world,
            ),
        );

        // expect each file's blob, no signed target, and the refusal
        const feed = join(directory, "feed", "targets");
        expect({
            targets: Object.keys(targets.signed.targets),
            blobs: (await world.files.readDirectory(join(feed, "blobs")))
                .map((entry) => entry.name)
                .toSorted(),
            refused,
        }).toEqual({
            targets: [],
            blobs: manifest.files.map((file) => file.digest).toSorted(),
            refused: [
                "BAD_REQUEST",
                "METADATA_INVALID",
                `the toolchain's files differ from its manifest: ${toolchain}`,
            ],
        });
    } finally {
        await world.files.remove(directory, { recursive: true });
    }
});
