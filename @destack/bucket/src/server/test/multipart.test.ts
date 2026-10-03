import { present } from "@destack/schema";
import { createHash } from "node:crypto";
import { TEST_DIALECTS } from "@destack/db/test";
import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { afterAll, afterEach, beforeAll, beforeEach, expect, test } from "@destack/test";
import { vi } from "vitest";
import { LEASE_LIFETIME } from "../bucket.ts";
import { BucketFixture, NOW, PRESIGNED } from "./fixture.ts";

/** The spaces database of each dialect for the scenarios' own spaces. */
let databases: Awaited<ReturnType<typeof BucketFixture.databases>>;

beforeAll(async () => {
    databases = await BucketFixture.databases();
});

afterAll(async () => {
    for (const database of databases.values()) {
        await database.close();
    }
});

beforeEach(() => {
    vi.useFakeTimers({ toFake: ["Date"], now: NOW });
});

afterEach(() => {
    vi.useRealTimers();
});

/** The smallest size of every multipart part but the last, five MiB. */
const PART_SIZE = 5 * 1024 * 1024;

/** Create, upload, complete and abort multipart uploads through presigned part transfers. */
test.for(TEST_DIALECTS)(
    "assemble a file from parts uploaded through presigned transfers on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client, bucket } = fixture;
        const key = "media/video.bin";

        // create the upload with the metadata its file receives
        const created = await client.createUpload({
            ...bucket,
            requestId: RequestId.create(),
            key,
            httpMetadata: { contentType: "application/octet-stream" },
            customMetadata: { source: "fixture" },
        });
        expect(created).toEqual({ uploadId: created.uploadId, key });

        // upload a full part and a final short part through separate transfers
        const first = new Uint8Array(PART_SIZE).fill(1);
        const last = new TextEncoder().encode("tail");
        const parts = [];
        for (const [index, body] of [first, last].entries()) {
            // presign the part, signing its length and selecting it in the query
            const signed = await client.uploadPart({
                ...bucket,
                key,
                uploadId: created.uploadId,
                partNumber: index + 1,
                size: body.byteLength,
            });
            const headers = { "content-length": String(body.byteLength) };
            const part = `partNumber=${index + 1}&uploadId=${created.uploadId}`;
            const url = `https://s3.test/files/media/video.bin?${part}`;
            expect(signed).toEqual({
                url: `https://s3.test/files/media/video.bin?${PRESIGNED}&X-Amz-SignedHeaders=content-length%3Bhost&${part}&X-Amz-Signature=${await fixture.sign("PUT", url, headers)}`,
                mode: "write",
                headers,
                expiresAt: NOW + LEASE_LIFETIME * 1000,
            });

            // upload the part and keep its entity tag
            const response = await fixture.transfer(signed, body);
            expect(response.status).toBe(200);
            parts.push({
                partNumber: index + 1,
                etag: present(response.headers.get("etag"), "etag"),
            });
        }

        // complete the upload into one file with the upload's metadata
        const completed = await client.completeUpload({
            ...bucket,
            requestId: RequestId.create(),
            key,
            uploadId: created.uploadId,
            parts,
        });
        const digests = [first, last].map((body) => createHash("md5").update(body).digest());
        const etag = `${createHash("md5").update(Buffer.concat(digests)).digest("hex")}-2`;
        expect(completed).toEqual({
            key,
            version: completed.version,
            size: PART_SIZE + 4,
            etag,
            uploaded: NOW,
            httpMetadata: { contentType: "application/octet-stream" },
            customMetadata: { source: "fixture" },
        });
        expect(await client.file({ ...bucket, key })).toEqual(completed);

        // download the assembled tail
        const download = await client.open({ mode: "read", ...bucket, key, range: "bytes=-4" });
        const tail = await fixture.transfer(download);
        expect({ status: tail.status, body: await tail.text() }).toEqual({
            status: 206,
            body: "tail",
        });
    },
);

/** Discard an aborted upload and refuse its later parts and completion. */
test.for(TEST_DIALECTS)("abort a multipart upload on %s", async (dialect) => {
    await using fixture = await BucketFixture.open(
        present(databases.get(dialect), "dialect").database,
    );
    const { client, bucket } = fixture;
    const key = "draft.bin";

    // abort an upload with one part
    const { uploadId } = await client.createUpload({
        ...bucket,
        requestId: RequestId.create(),
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
            requestId: RequestId.create(),
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
    await expect(
        client.completeUpload({
            ...bucket,
            requestId: RequestId.create(),
            key,
            uploadId,
            parts: [{ partNumber: 1, etag }],
        }),
    ).rejects.toEqual(
        new ServiceError("NOT_FOUND", {
            message: "multipart upload does not exist",
            defined: true,
        }),
    );
    expect(await client.file({ ...bucket, key })).toBeNull();
});
