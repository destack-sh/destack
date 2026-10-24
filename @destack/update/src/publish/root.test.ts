import { UpdateFixture } from "../test/index.ts";
import { expect, test, refusal } from "@destack/test";
import { SigningKey } from "./key.ts";
import { TrustedRoot } from "./root.ts";
import { Metadata, MetadataKind } from "@tufjs/models";

test("require independent RSA root signatures and both quorums during rotation", async () => {
    // run in a world paused at the system's time, near the wall clock TUF checks expiry on
    const world = UpdateFixture.world().environment("test");

    // exercise the same RSA-PSS scheme used by the hardware without persisting private keys
    const first = await SigningKey.generate("rsassa-pss-sha256", world.crypto);
    const second = await SigningKey.generate("rsassa-pss-sha256", world.crypto);
    const third = await SigningKey.generate("rsassa-pss-sha256", world.crypto);
    const roots = [first, second, third];
    const keys = {
        targets: (await SigningKey.generate("ed25519", world.crypto)).public,
        snapshot: (await SigningKey.generate("ed25519", world.crypto)).public,
        timestamp: (await SigningKey.generate("ed25519", world.crypto)).public,
    };
    const expires = new Date(world.clock.now() + 365 * 86_400_000).toISOString();
    const root = TrustedRoot.create(
        1,
        roots.map((key) => key.public),
        keys,
        expires,
        world,
    );
    expect(() => TrustedRoot.verify(root)).toThrow("root was signed by 0/2 keys");

    // a repeated signature from one key never satisfies the quorum
    await first.sign(root);
    await first.sign(root);
    expect(() => TrustedRoot.verify(root)).toThrow("root was signed by 1/2 keys");
    await second.sign(root);
    TrustedRoot.verify(root);

    // replacing one lost key requires two surviving keys and the replacement quorum
    const replacement = await SigningKey.generate("ed25519", world.crypto);
    const rotated = TrustedRoot.create(
        2,
        [second.public, third.public, replacement.public],
        keys,
        expires,
        world,
    );
    await second.sign(rotated);
    await replacement.sign(rotated);
    expect(() => TrustedRoot.verify(rotated, root)).toThrow("root was signed by 1/2 keys");
    await third.sign(rotated);
    TrustedRoot.verify(rotated, root);
    expect(await refusal(() => TrustedRoot.verify(rotated))).toEqual([
        "BAD_REQUEST",
        "INVALID_METADATA",
        "expected root version 1",
    ]);

    // changing signed content invalidates every signature
    const changed = Metadata.fromJSON(MetadataKind.Root, {
        ...rotated.toJSON(),
        signed: {
            ...rotated.signed.toJSON(),
            expires: new Date(world.clock.now() + 366 * 86_400_000).toISOString(),
        },
    });
    expect(() => TrustedRoot.verify(changed, root)).toThrow("root was signed by 0/2 keys");
    expect(
        await refusal(() =>
            TrustedRoot.create(1, [first.public, first.public, third.public], keys, expires, world),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "INVALID_METADATA",
        "root requires three independent keys and three distinct online keys",
    ]);
}, 5000);
