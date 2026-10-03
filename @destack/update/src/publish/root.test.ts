import { expect, test } from "@destack/test";
import { generateKeyPairSync } from "node:crypto";
import { SigningKey } from "./key.ts";
import { TrustedRoot } from "./root.ts";
import { Metadata, MetadataKind } from "@tufjs/models";

/** Generate an RSA-PSS root key as the hardware keeps it. */
function generateRsaKey(): SigningKey {
    const pair = generateKeyPairSync("rsa", { modulusLength: 2048 });

    return new SigningKey(pair.privateKey.export({ format: "pem", type: "pkcs8" }));
}

test("require independent RSA root signatures and both quorums during rotation", () => {
    // exercise the same RSA-PSS scheme used by the hardware without persisting private keys
    const first = generateRsaKey();
    const second = generateRsaKey();
    const third = generateRsaKey();
    const roots = [first, second, third];
    const keys = {
        targets: SigningKey.generate().public,
        snapshot: SigningKey.generate().public,
        timestamp: SigningKey.generate().public,
    };
    const expires = new Date(Date.now() + 365 * 86_400_000).toISOString();
    const root = TrustedRoot.create(
        1,
        roots.map((key) => key.public),
        keys,
        expires,
    );
    expect(() => TrustedRoot.verify(root)).toThrow("root was signed by 0/2 keys");

    // a repeated signature from one key never satisfies the quorum
    root.sign((bytes) => first.sign(bytes));
    root.sign((bytes) => first.sign(bytes));
    expect(() => TrustedRoot.verify(root)).toThrow("root was signed by 1/2 keys");
    root.sign((bytes) => second.sign(bytes));
    TrustedRoot.verify(root);

    // replacing one lost key requires two surviving keys and the replacement quorum
    const replacement = SigningKey.generate();
    const rotated = TrustedRoot.create(
        2,
        [second.public, third.public, replacement.public],
        keys,
        expires,
    );
    rotated.sign((bytes) => second.sign(bytes));
    rotated.sign((bytes) => replacement.sign(bytes));
    expect(() => TrustedRoot.verify(rotated, root)).toThrow("root was signed by 1/2 keys");
    rotated.sign((bytes) => third.sign(bytes));
    TrustedRoot.verify(rotated, root);
    expect(() => TrustedRoot.verify(rotated)).toThrow("expected root version 1");

    // changing signed content invalidates every signature
    const changed = Metadata.fromJSON(MetadataKind.Root, {
        ...rotated.toJSON(),
        signed: {
            ...rotated.signed.toJSON(),
            expires: new Date(Date.now() + 366 * 86_400_000).toISOString(),
        },
    });
    expect(() => TrustedRoot.verify(changed, root)).toThrow("root was signed by 0/2 keys");
    expect(() =>
        TrustedRoot.create(1, [first.public, first.public, third.public], keys, expires),
    ).toThrow("root requires three independent keys and three distinct online keys");
}, 5000);
