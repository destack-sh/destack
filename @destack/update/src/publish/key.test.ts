import { generateKeyPairSync } from "node:crypto";
import { expect, refusal, test } from "@destack/test";
import { readPublicKey } from "./key.ts";

test("refuse an RSA signing key shorter than 2048 bits", async () => {
    const { publicKey } = generateKeyPairSync("rsa", { modulusLength: 1024 });
    const pem = publicKey.export({ format: "pem", type: "spki" });

    expect(await refusal(() => readPublicKey(pem))).toEqual([
        "BAD_REQUEST",
        "INVALID_SIGNING_KEY",
        "RSA signing keys must contain at least 2048 bits",
    ]);
});
