import { expect, test } from "@destack/test";
import { IdentityError } from "../error/error.ts";
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
