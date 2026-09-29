import { expect, test } from "@destack/test";
import { generateKeyPairSync } from "node:crypto";
import { SigningKey } from "./key.ts";
import { createRoot, verifyRoot } from "./root.ts";
import { Metadata, MetadataKind } from "@tufjs/models";

test("require independent RSA root signatures and both quorums during rotation", () => {
    // exercise the same RSA-PSS scheme used by the hardware without persisting private keys
    const roots = Array.from({ length: 3 }, () => {
        const pair = generateKeyPairSync("rsa", { modulusLength: 2048 });

        return new SigningKey(pair.privateKey.export({ format: "pem", type: "pkcs8" }).toString());
    });
    const keys = {
        targets: SigningKey.generate().public,
        snapshot: SigningKey.generate().public,
        timestamp: SigningKey.generate().public,
    };
    const expires = new Date(Date.now() + 365 * 86_400_000).toISOString();
    const root = createRoot(
        1,
        roots.map((key) => key.public),
        keys,
        expires,
    );
    expect(() => verifyRoot(root)).toThrow("root was signed by 0/2 keys");

    // a repeated signature from one key never satisfies the quorum
    root.sign((bytes) => roots[0]!.sign(bytes));
    root.sign((bytes) => roots[0]!.sign(bytes));
    expect(() => verifyRoot(root)).toThrow("root was signed by 1/2 keys");
    root.sign((bytes) => roots[1]!.sign(bytes));
    verifyRoot(root);
    expect(root.signed.roles.root?.threshold).toBe(2);
    expect(Object.keys(root.signatures).sort()).toEqual(
        roots
            .slice(0, 2)
            .map((key) => key.public.keyID)
            .sort(),
    );

    // replacing one lost key requires two surviving keys and the replacement quorum
    const replacement = SigningKey.generate();
    const rotated = createRoot(
        2,
        [roots[1]!.public, roots[2]!.public, replacement.public],
        keys,
        expires,
    );
    rotated.sign((bytes) => roots[1]!.sign(bytes));
    rotated.sign((bytes) => replacement.sign(bytes));
    expect(() => verifyRoot(rotated, root)).toThrow("root was signed by 1/2 keys");
    rotated.sign((bytes) => roots[2]!.sign(bytes));
    verifyRoot(rotated, root);
    expect(() => verifyRoot(rotated)).toThrow("expected root version 1");

    // changing signed content invalidates every signature
    const changed = Metadata.fromJSON(MetadataKind.Root, {
        ...rotated.toJSON(),
        signed: {
            ...rotated.signed.toJSON(),
            expires: new Date(Date.now() + 366 * 86_400_000).toISOString(),
        },
    });
    expect(() => verifyRoot(changed, root)).toThrow("root was signed by 0/2 keys");
    expect(() =>
        createRoot(1, [roots[0]!.public, roots[0]!.public, roots[2]!.public], keys, expires),
    ).toThrow("root requires three independent keys and three distinct online keys");
}, 5000);
