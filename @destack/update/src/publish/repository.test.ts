import { UpdateFixture } from "../test/index.ts";
import { expect, test } from "@destack/test";
import { Metadata, MetadataKind, Targets } from "@tufjs/models";
import { join } from "node:path";
import { SigningKey } from "./key.ts";
import { TrustedRoot } from "./root.ts";
import { parseObject } from "./json.ts";
import { SignedRepository } from "./repository.ts";
import { COMMIT } from "../test/index.ts";

test("renew freshness while preserving exact targets authorization", async () => {
    // run in a world paused at the system's time, near the wall clock TUF checks expiry on
    const world = UpdateFixture.world().environment("test");

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
                    target: "aarch64-apple-darwin",
                    version: "2026.9.1",
                    commit: COMMIT,
                    archive,
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
