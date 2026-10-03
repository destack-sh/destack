import { present } from "@destack/schema";
import { createHash } from "node:crypto";
import { Readable } from "node:stream";
import { GetObjectCommand, PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { expect, test } from "@destack/test";
import { SignatureV4 } from "./signature.ts";
import { CREDENTIALS, failure, REGION, S3Fixture } from "./test/fixture.ts";

test("accept the AWS SDK's default checksums, trailers included", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client({ endpoint: "https://s3.test", checksums: "WHEN_SUPPORTED" });

    // store a buffered body with a CRC32 header and a streamed body with a trailing CRC32
    await client.send(
        new PutObjectCommand({ Bucket: "files", Key: "buffered.txt", Body: "buffered" }),
    );
    await client.send(
        new PutObjectCommand({
            Bucket: "files",
            Key: "streamed.txt",
            Body: Readable.from([new TextEncoder().encode("streamed contents")]),
            ContentLength: 17,
        }),
    );
    const bucket = present(fixture.buckets.get("files"), "files");
    expect({
        payloadHashes: fixture.payloadHashes,
        bodies: [
            await present(await bucket.get("buffered.txt"), "buffered.txt").text(),
            await present(await bucket.get("streamed.txt"), "streamed.txt").text(),
        ],
    }).toEqual({
        payloadHashes: [
            createHash("sha256").update("buffered").digest("hex"),
            "STREAMING-UNSIGNED-PAYLOAD-TRAILER",
        ],
        bodies: ["buffered", "streamed contents"],
    });
});

test("follow presigned GET and PUT URLs and refuse forged or expired ones", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();

    // upload through a presigned PUT carrying user metadata in its query
    const upload = await getSignedUrl(
        client,
        new PutObjectCommand({
            Bucket: "files",
            Key: "shared.txt",
            ContentType: "text/plain",
            Metadata: { via: "url" },
        }),
        { expiresIn: 60 },
    );
    const put = await fixture.fetch(upload, {
        method: "PUT",
        body: "presigned",
        headers: { "content-type": "text/plain" },
    });
    expect(put.status).toBe(200);

    // download it through a presigned GET
    const download = await getSignedUrl(
        client,
        new GetObjectCommand({ Bucket: "files", Key: "shared.txt" }),
        { expiresIn: 60 },
    );
    const get = await fixture.fetch(download);
    expect({
        status: get.status,
        type: get.headers.get("content-type"),
        via: get.headers.get("x-amz-meta-via"),
        body: await get.text(),
    }).toEqual({
        status: 200,
        type: "text/plain",
        via: "url",
        body: "presigned",
    });

    // refuse an expired URL, a URL for another key and a wrong secret
    const expired = await getSignedUrl(
        client,
        new GetObjectCommand({ Bucket: "files", Key: "shared.txt" }),
        {
            expiresIn: 60,
            signingDate: new Date(Date.now() - 120_000),
        },
    );
    const forged = await fixture.fetch(download.replace("shared.txt", "other.txt"));
    const codes = await Promise.all(
        [await fixture.fetch(expired), forged].map(async (response) => ({
            status: response.status,
            code: /<Code>(\w+)<\/Code>/u.exec(await response.text())?.[1],
        })),
    );
    expect(codes).toEqual([
        { status: 403, code: "AccessDenied" },
        { status: 403, code: "SignatureDoesNotMatch" },
    ]);
    expect(
        await failure(
            fixture
                .client({ secretAccessKey: "wrong" })
                .send(new GetObjectCommand({ Bucket: "files", Key: "shared.txt" })),
        ),
    ).toEqual({
        name: "SignatureDoesNotMatch",
        status: 403,
    });
});

test("answer failures with S3 error documents", async () => {
    await using fixture = await S3Fixture.open();

    // refuse anonymous requests and unknown access keys
    const anonymous = await fixture.fetch("http://s3.test/files/readme.txt");
    const requestId = anonymous.headers.get("x-amz-request-id");
    expect({ status: anonymous.status, body: await anonymous.text() }).toEqual({
        status: 403,
        body: `<?xml version="1.0" encoding="UTF-8"?><Error><Code>AccessDenied</Code><Message>anonymous requests are not allowed</Message><Resource>/files/readme.txt</Resource><RequestId>${requestId}</RequestId></Error>`,
    });
    const client = new S3Client({
        region: REGION,
        endpoint: "http://s3.test",
        forcePathStyle: true,
        maxAttempts: 1,
        credentials: { accessKeyId: "AKIDUNKNOWN", secretAccessKey: "secret" },
    });
    const unknown = await getSignedUrl(
        client,
        new GetObjectCommand({ Bucket: "files", Key: "readme.txt" }),
        { expiresIn: 60 },
    );
    const response = await fixture.fetch(unknown);
    expect({
        status: response.status,
        code: /<Code>(\w+)<\/Code>/u.exec(await response.text())?.[1],
    }).toEqual({
        status: 403,
        code: "InvalidAccessKeyId",
    });
});

test("answer CORS preflights and expose headers to allowed origins", async () => {
    await using fixture = await S3Fixture.open();

    // allow a matching origin, method and headers
    const preflight = await fixture.fetch("http://s3.test/files/readme.txt", {
        method: "OPTIONS",
        headers: {
            origin: "https://app.example.com",
            "access-control-request-method": "PUT",
            "access-control-request-headers": "Content-Type, X-Amz-Date",
        },
    });
    const { "x-amz-request-id": _, ...headers } = Object.fromEntries(preflight.headers);
    expect({ status: preflight.status, headers }).toEqual({
        status: 200,
        headers: {
            "access-control-allow-origin": "https://app.example.com",
            "access-control-allow-methods": "GET, PUT",
            "access-control-allow-headers": "content-type, x-amz-date",
            "access-control-expose-headers": "etag",
            "access-control-max-age": "600",
            vary: "Origin, Access-Control-Request-Headers, Access-Control-Request-Method",
        },
    });

    // refuse another origin, and grant the allowed origin on actual responses
    const refused = await fixture.fetch("http://s3.test/files/readme.txt", {
        method: "OPTIONS",
        headers: { origin: "https://other.test", "access-control-request-method": "GET" },
    });
    expect({
        status: refused.status,
        code: /<Code>(\w+)<\/Code>/u.exec(await refused.text())?.[1],
    }).toEqual({
        status: 403,
        code: "AccessForbidden",
    });
    const actual = await fixture.fetch("http://s3.test/files/readme.txt", {
        headers: { origin: "https://app.example.com" },
    });
    expect({
        status: actual.status,
        origin: actual.headers.get("access-control-allow-origin"),
        expose: actual.headers.get("access-control-expose-headers"),
    }).toEqual({ status: 403, origin: "https://app.example.com", expose: "etag" });
});

test("refuse bodies whose signed hash or checksum differs, storing nothing", async () => {
    await using fixture = await S3Fixture.open();
    const signer = new SignatureV4({ region: REGION });

    // sign writes declaring the hash or CRC32 of other contents
    const put = (key: string, headers: Record<string, string>) =>
        signer.sign(
            new Request(`http://s3.test/files/${key}`, { method: "PUT", body: "actual", headers }),
            CREDENTIALS,
            Date.now(),
        );
    const requests = [
        await put("hashed.txt", {
            "x-amz-content-sha256": createHash("sha256").update("other").digest("hex"),
        }),
        await put("checksummed.txt", { "x-amz-checksum-crc32": "AAAAAA==" }),
    ];

    // refuse each after reading its body, leaving both keys absent
    const responses = await Promise.all(
        requests.map(async (request) => {
            const response = await fixture.server.fetch(request);

            return {
                status: response.status,
                code: /<Code>(\w+)<\/Code>/u.exec(await response.text())?.[1],
            };
        }),
    );
    expect(responses).toEqual([
        { status: 400, code: "XAmzContentSHA256Mismatch" },
        { status: 400, code: "BadDigest" },
    ]);
    expect(await present(fixture.buckets.get("files"), "files").list()).toEqual({
        files: [],
        delimitedPrefixes: [],
        truncated: false,
    });
});
