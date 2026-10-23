import { createHash } from "node:crypto";
import { expect, test, refusal } from "@destack/test";
import { customerKeyHeaders } from "./headers.ts";
import { S3Request } from "./request.ts";
import type { S3Authorization } from "./signature.ts";

test("write a customer key's SSE-C headers, which a request reads back as the key", async () => {
    const key = new Uint8Array(32).fill(7);
    const headers = customerKeyHeaders(key.toBase64());
    const authorization: S3Authorization = {
        accessKeyId: "tester",
        payloadHash: "UNSIGNED-PAYLOAD",
    };
    const request = new S3Request(
        new Request("https://s3.test/files/sealed.txt", { headers }),
        "files",
        "sealed.txt",
        authorization,
    );

    expect([headers, request.ssecKey()]).toEqual([
        {
            "x-amz-server-side-encryption-customer-algorithm": "AES256",
            "x-amz-server-side-encryption-customer-key": key.toBase64(),
            "x-amz-server-side-encryption-customer-key-md5": createHash("md5")
                .update(key)
                .digest("base64"),
        },
        key.toHex(),
    ]);
});

test("refuse writing the headers of a key other than 32 bytes of base64", async () => {
    expect(await refusal(() => customerKeyHeaders(new Uint8Array(16).toBase64()))).toEqual([
        "BAD_REQUEST",
        "INVALID_ARGUMENT",
        "the customer key must be 32 bytes of base64",
    ]);
});
