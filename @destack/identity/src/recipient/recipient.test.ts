import { expect, test, refusal } from "@destack/test";
import { decodeProtectedHeader } from "jose";
import { Derivation } from "../derivation/derivation.ts";
import { Recipient } from "./recipient.ts";

test("seal bytes to a recipient, which alone opens them under the same context", async () => {
    const recipient = await Recipient.generate();
    const context = { label: "@destack/identity/test", context: "vault/secret-1/1" };
    const other = { label: "@destack/identity/test", context: "vault/secret-2/1" };
    const sealed = await Recipient.of(recipient.key).seal(new Uint8Array([1, 2, 3]), context);

    // open only on the recipient under the sealed context
    expect([...(await recipient.open(sealed, context))]).toEqual([1, 2, 3]);
    const failed = [
        "INTERNAL_SERVER_ERROR",
        "DECRYPTION_FAILED",
        "ciphertext authentication failed",
    ];
    expect(await refusal(recipient.open(sealed, other))).toEqual(failed);
    expect(await refusal((await Recipient.generate()).open(sealed, context))).toEqual(failed);
    await expect(Recipient.of(recipient.key).open(sealed, context)).rejects.toThrow(
        new TypeError("the recipient has no private key to decrypt with"),
    );
});

test("derive the same recipient of a root under a label, another under another label or root, and seal to it by its public key", async () => {
    // derive under two roots and two labels
    const root = Derivation.of(await Derivation.root(new Uint8Array(32).fill(7)));
    const other = Derivation.of(await Derivation.root(new Uint8Array(32).fill(8)));
    const [first, again, relabelled, rerooted] = await Promise.all([
        Recipient.derive(root, { label: "message", context: "" }),
        Recipient.derive(root, { label: "message", context: "" }),
        Recipient.derive(root, { label: "relay", context: "" }),
        Recipient.derive(other, { label: "message", context: "" }),
    ]);

    // open on a derivation of the same root and label what its public key sealed, as ECDH-ES+A256KW
    const context = { label: "@destack/identity/test", context: "message space-1" };
    const sealed = await Recipient.of(first.key).seal(new Uint8Array([4, 5]), context);

    expect({
        keys: [first.key === again.key, first.key === relabelled.key, first.key === rerooted.key],
        algorithm: decodeProtectedHeader(sealed).alg,
        opened: [...(await again.open(sealed, context))],
    }).toEqual({ keys: [true, false, false], algorithm: "ECDH-ES+A256KW", opened: [4, 5] });
});
