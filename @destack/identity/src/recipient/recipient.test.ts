import { expect, test } from "@destack/test";
import { decodeProtectedHeader } from "jose";
import { IdentityError } from "../error/error.ts";
import { Derivation } from "../derivation/derivation.ts";
import { Recipient } from "./recipient.ts";

test("seal bytes to a recipient, which alone opens them under the same context", async () => {
    const recipient = await Recipient.generate();
    const context = new TextEncoder().encode("vault/secret-1/1");
    const other = new TextEncoder().encode("vault/secret-2/1");
    const sealed = await Recipient.of(recipient.key).seal(new Uint8Array([1, 2, 3]), context);

    // open only on the recipient under the sealed context
    expect([...(await recipient.open(sealed, context))]).toEqual([1, 2, 3]);
    const failed = new IdentityError("DECRYPTION_FAILED", "ciphertext authentication failed");
    await expect(recipient.open(sealed, other)).rejects.toEqual(failed);
    await expect((await Recipient.generate()).open(sealed, context)).rejects.toEqual(failed);
    await expect(Recipient.of(recipient.key).open(sealed, context)).rejects.toThrow(
        new TypeError("open sealed bytes on the recipient holding its private key"),
    );
});

test("derive the same recipient of a root under a label, another under another label or root, and seal to it by its public key", async () => {
    // derive under two roots and two labels
    const root = Derivation.of(await Derivation.root(new Uint8Array(32).fill(7)));
    const other = Derivation.of(await Derivation.root(new Uint8Array(32).fill(8)));
    const [first, again, relabelled, rerooted] = await Promise.all([
        Recipient.derive(root, "message"),
        Recipient.derive(root, "message"),
        Recipient.derive(root, "relay"),
        Recipient.derive(other, "message"),
    ]);

    // open on a derivation of the same root and label what its public key sealed, as ECDH-ES+A256KW
    const context = new TextEncoder().encode("message space-1");
    const sealed = await Recipient.of(first.key).seal(new Uint8Array([4, 5]), context);

    expect({
        keys: [first.key === again.key, first.key === relabelled.key, first.key === rerooted.key],
        algorithm: decodeProtectedHeader(sealed).alg,
        opened: [...(await again.open(sealed, context))],
    }).toEqual({ keys: [true, false, false], algorithm: "ECDH-ES+A256KW", opened: [4, 5] });
});
