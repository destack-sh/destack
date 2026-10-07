import { createHash } from "node:crypto";
import { expect, test } from "@destack/test";
import { S3Error } from "./error.ts";
import { customerKeyHeaders, S3Request } from "./request.ts";
import type { S3Authorization } from "./signature.ts";

test("write a customer key's SSE-C headers, which a request reads back as the key", async () => {
    const key = new Uint8Array(32).fill(7);
    const headers = customerKeyHeaders(key.toBase64());
    const authorization: S3Authorization = {
        accessKeyId: "tester",
        sign: async () => "0".repeat(64),
        date: "20260929T100000Z",
        scope: "20260929/auto/s3/aws4_request",
        signature: "0".repeat(64),
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

test("refuse writing the headers of a key other than 32 bytes of base64", () => {
    expect(() => customerKeyHeaders(new Uint8Array(16).toBase64())).toThrow(
        new S3Error("InvalidArgument", "the customer key must be 32 bytes of base64"),
    );
});
