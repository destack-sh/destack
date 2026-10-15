import { AwsSigner } from "@destack/identity/aws";
import { expect, test } from "@destack/test";
import { ConnectionRequest } from "./request.ts";

/** The time requests are lent at: 2026-10-06T12:00:00Z. */
const NOW = Date.parse("2026-10-06T12:00:00Z");

test("send a key in its header with its scheme, a token as Bearer, and a client certificate beside the request", async () => {
    const request = new Request("https://api.test/items");
    const key = await ConnectionRequest.of(
        { kind: "key", header: "authorization", scheme: "Token" },
        { material: { kind: "key", key: "k1" } },
        request,
        NOW,
    );
    const bare = await ConnectionRequest.of(
        { kind: "key", header: "x-api-key" },
        { material: { kind: "key", key: "k2" } },
        request,
        NOW,
    );
    const token = await ConnectionRequest.of(
        { kind: "client-credentials" },
        { token: { accessToken: "t1", tokenType: "bearer" } },
        request,
        NOW,
    );
    const mtls = await ConnectionRequest.of(
        { kind: "mtls" },
        { material: { kind: "mtls", certificate: "CERT", privateKey: "KEY" } },
        request,
        NOW,
    );

    expect([
        key.request.headers.get("authorization"),
        bare.request.headers.get("x-api-key"),
        token.request.headers.get("authorization"),
        [mtls.certificate, mtls.request.headers.get("authorization")],
    ]).toEqual(["Token k1", "k2", "Bearer t1", [{ certificate: "CERT", privateKey: "KEY" }, null]]);
});

test("sign a request's method, target URI and content digest with HMAC-SHA256 (RFC 9421, RFC 9530)", async () => {
    // sign a post with a shared secret
    const secret = crypto.getRandomValues(new Uint8Array(32));
    const loan = await ConnectionRequest.of(
        { kind: "signature", algorithm: "hmac-sha256", keyId: "key-1" },
        {
            material: {
                kind: "signature",
                key: secret.toBase64({ alphabet: "base64url", omitPadding: true }),
            },
        },
        new Request("https://api.test/items?page=2", { method: "POST", body: '{"name":"a"}' }),
        NOW,
    );

    // verify the signature over the base RFC 9421 2.5 writes
    const digest = new Uint8Array(
        await crypto.subtle.digest("SHA-256", new TextEncoder().encode('{"name":"a"}')),
    ).toBase64();
    const parameters =
        '("@method" "@target-uri" "content-digest");created=1791288000;keyid="key-1";alg="hmac-sha256"';
    const base = [
        '"@method": POST',
        '"@target-uri": https://api.test/items?page=2',
        `"content-digest": sha-256=:${digest}:`,
        `"@signature-params": ${parameters}`,
    ].join("\n");
    const key = await crypto.subtle.importKey(
        "raw",
        secret,
        { name: "HMAC", hash: "SHA-256" },
        false,
        ["verify"],
    );
    const signature = Uint8Array.fromBase64(
        String(loan.request.headers.get("signature")).slice("sig1=:".length, -1),
    );

    expect({
        input: loan.request.headers.get("signature-input"),
        digest: loan.request.headers.get("content-digest"),
        isValid: await crypto.subtle.verify("HMAC", key, signature, new TextEncoder().encode(base)),
        body: await loan.request.text(),
    }).toEqual({
        input: `sig1=${parameters}`,
        digest: `sha-256=:${digest}:`,
        isValid: true,
        body: '{"name":"a"}',
    });
});

test("sign a request with AWS Signature Version 4 over its body, encoding an S3 path once", async () => {
    const credentials = { accessKeyId: "AKIDEXAMPLE", secretAccessKey: "secret" };
    const loan = await ConnectionRequest.of(
        { kind: "sigv4", region: "eu-central-1", service: "s3" },
        { material: { kind: "sigv4", ...credentials } },
        new Request("https://bucket.s3.test/a b.txt", {
            method: "PUT",
            headers: { "content-type": "text/plain" },
            body: "hello",
        }),
        NOW,
    );

    // sign the same request with the signer directly
    const payloadHash = await AwsSigner.payloadHash("hello");
    const expected = await new AwsSigner({
        region: "eu-central-1",
        service: "s3",
        credentials,
    }).authorize(
        {
            method: "PUT",
            path: "/a%20b.txt",
            query: [],
            headers: {
                host: "bucket.s3.test",
                "content-type": "text/plain",
                "x-amz-content-sha256": payloadHash,
            },
            payloadHash,
        },
        new Date(NOW),
    );

    expect([loan.request.headers.get("authorization"), await loan.request.text()]).toEqual([
        expected["authorization"],
        "hello",
    ]);
});
