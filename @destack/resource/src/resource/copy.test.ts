import { expect, test } from "@destack/test";
import { Recipient } from "./index.ts";

test("seal bytes to a recipient, which alone opens them under the same context", async () => {
    const recipient = await Recipient.generate();
    const context = new TextEncoder().encode("vault/secret-1/1");
    const other = new TextEncoder().encode("vault/secret-2/1");
    const sealed = await Recipient.of(recipient.key).seal(new Uint8Array([1, 2, 3]), context);

    // open under the sealed context, and refuse another context, another recipient and the public half alone
    expect([...(await recipient.open(sealed, context))]).toEqual([1, 2, 3]);
    await expect(recipient.open(sealed, other)).rejects.toMatchObject({ name: "OperationError" });
    await expect((await Recipient.generate()).open(sealed, context)).rejects.toMatchObject({
        name: "OperationError",
    });
    await expect(Recipient.of(recipient.key).open(sealed, context)).rejects.toThrow(
        new TypeError("open sealed bytes on the recipient holding its private key"),
    );
});
