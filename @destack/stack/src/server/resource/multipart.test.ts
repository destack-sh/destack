import { TestDatabase } from "@destack/db/test";
import { present } from "@destack/schema";
import { RequestId } from "@destack/sync";
import {
    afterAll,
    afterEach,
    beforeAll,
    beforeEach,
    expect,
    refusal,
    test,
    vi,
} from "@destack/test";
import { LEASE_LIFETIME_SECONDS } from "@destack/bucket/s3";
import { BucketFixture, NOW, PRESIGNED } from "../../test/index.ts";
import { Simulation } from "@destack/environment/simulation";

/** The world the scenarios run in, its clock at the presigning time. */
const world = new Simulation({ seed: 1, start: NOW }).environment("space");

/** The spaces database for the scenarios' own spaces. */
let storage: TestDatabase;

beforeAll(async () => {
    storage = await BucketFixture.database(world);
});

afterAll(async () => {
    await storage.close();
});

beforeEach(() => {
    vi.useFakeTimers({ toFake: ["Date"], now: NOW });
});

afterEach(() => {
    vi.useRealTimers();
});

/** Create and complete a multipart upload through a presigned part transfer. */
test("create, lease and complete a multipart upload into a file with the upload's metadata", async () => {
    await using fixture = await BucketFixture.open(storage.database, world);
    const { client, bucket } = fixture;
    const key = "media/video.bin";

    // create the upload with the metadata its file receives
    const created = await client.createUpload({
        ...bucket,
        requestId: RequestId.create(fixture.environment),
        key,
        httpMetadata: { contentType: "application/octet-stream" },
        customMetadata: { source: "fixture" },
    });
    expect(created).toEqual({ uploadId: created.uploadId, key });

    // lease the one part, signing its length and selecting it in the query
    const body = new TextEncoder().encode("tail");
    const signed = await client.uploadPart({
        ...bucket,
        key,
        uploadId: created.uploadId,
        partNumber: 1,
        size: body.byteLength,
    });
    const headers = { "content-length": String(body.byteLength) };
    const part = `partNumber=1&uploadId=${created.uploadId}`;
    const url = `https://s3.test/files/media/video.bin?${part}`;
    expect(signed).toEqual({
        url: `https://s3.test/files/media/video.bin?${PRESIGNED}&X-Amz-SignedHeaders=content-length%3Bhost&${part}&X-Amz-Signature=${await fixture.sign("PUT", url, headers)}`,
        mode: "write",
        headers,
        expiresAt: NOW + LEASE_LIFETIME_SECONDS * 1000,
    });

    // upload the part, then complete the upload into one file with the upload's metadata
    const response = await fixture.transfer(signed, body);
    const etag = present(response.headers.get("etag"), "etag");
    const completed = await client.completeUpload({
        ...bucket,
        requestId: RequestId.create(fixture.environment),
        key,
        uploadId: created.uploadId,
        parts: [{ partNumber: 1, etag }],
    });
    expect([response.status, completed, await client.file({ ...bucket, key })]).toEqual([
        200,
        {
            key,
            version: completed.version,
            size: body.byteLength,
            etag: completed.etag,
            uploaded: NOW,
            httpMetadata: { contentType: "application/octet-stream" },
            customMetadata: { source: "fixture" },
        },
        completed,
    ]);
});

/** Discard an aborted upload and refuse its later parts and completion. */
test("abort a multipart upload", async () => {
    await using fixture = await BucketFixture.open(storage.database, world);
    const { client, bucket } = fixture;
    const key = "draft.bin";

    // abort an upload with one part
    const { uploadId } = await client.createUpload({
        ...bucket,
        requestId: RequestId.create(fixture.environment),
        key,
        httpMetadata: {},
        customMetadata: {},
    });
    const part = { ...bucket, key, uploadId, partNumber: 1, size: 5 };
    const uploaded = await fixture.transfer(await client.uploadPart(part), "draft");
    const etag = present(uploaded.headers.get("etag"), "etag");
    expect(
        await client.abortUpload({
            ...bucket,
            requestId: RequestId.create(fixture.environment),
            key,
            uploadId,
        }),
    ).toEqual({});

    // refuse parts and completion of the aborted upload
    const late = await fixture.transfer(await client.uploadPart(part), "later");
    const requestId = late.headers.get("x-amz-request-id");
    expect({ status: late.status, body: await late.text() }).toEqual({
        status: 404,
        body: `<?xml version="1.0" encoding="UTF-8"?><Error><Code>NoSuchUpload</Code><Message>multipart upload does not exist</Message><Resource>/files/draft.bin</Resource><RequestId>${requestId}</RequestId></Error>`,
    });
    expect(
        await refusal(
            client.completeUpload({
                ...bucket,
                requestId: RequestId.create(fixture.environment),
                key,
                uploadId,
                parts: [{ partNumber: 1, etag }],
            }),
        ),
    ).toEqual(["NOT_FOUND", "NO_SUCH_UPLOAD", "multipart upload does not exist"]);
    expect(await client.file({ ...bucket, key })).toBeNull();
});
