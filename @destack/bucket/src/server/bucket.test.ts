import { createHash } from "node:crypto";
import { principal } from "@destack/access";
import { invokeService } from "@destack/audit";
import { eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { ServiceError } from "@destack/service/error";
import { aligned, schema, present } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { afterAll, afterEach, beforeAll, beforeEach, expect, test } from "@destack/test";
import { vi } from "vitest";
import { v7 } from "uuid";
import { spaceService } from "@destack/space/service";
import * as bucketObject from "../object/index.ts";
import { LEASE_LIFETIME } from "./bucket.ts";
import { BucketFixture, NOW, PRESIGNED } from "../test/index.ts";

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

/** Upload through a presigned transfer, list, head, download ranges and remove files. */
test.for(TEST_DIALECTS)(
    "transfer files through presigned URLs and read their metadata on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client, bucket } = fixture;

        // presign an upload that signs its length and type, and moves its metadata into the query
        const upload = await client.open({
            mode: "write",
            ...bucket,
            key: "documents/readme.txt",
            size: 13,
            httpMetadata: { contentType: "text/plain" },
            customMetadata: { author: "fixture" },
        });
        expect(upload).toEqual({
            url: `https://s3.test/files/documents/readme.txt?${PRESIGNED}&X-Amz-SignedHeaders=content-length%3Bcontent-type%3Bhost&x-amz-meta-author=fixture&X-Amz-Signature=b8cf00f29b8401175b9263e153cbda50f6695e2ec7ea06138540f29f19c7c0b0`,
            mode: "write",
            headers: { "content-length": "13", "content-type": "text/plain" },
            expiresAt: NOW + LEASE_LIFETIME * 1000,
        });

        // store the body through the transfer, answering its entity tag
        const etag = createHash("md5").update("Hello, bucket").digest("hex");
        const stored = await fixture.transfer(upload, "Hello, bucket");
        expect({ status: stored.status, headers: Object.fromEntries(stored.headers) }).toEqual({
            status: 200,
            headers: {
                etag: `"${etag}"`,
                "x-amz-request-id": present(stored.headers.get("x-amz-request-id"), "id"),
            },
        });
        const nested = await client.open({
            mode: "write",
            ...bucket,
            key: "documents/notes/first.txt",
            size: 5,
            httpMetadata: {},
            customMetadata: {},
        });
        expect((await fixture.transfer(nested, "notes")).status).toBe(200);

        // read the stored metadata, and list it beside the nested prefix
        const head = present(await client.file({ ...bucket, key: "documents/readme.txt" }), "head");
        expect(head).toEqual({
            key: "documents/readme.txt",
            version: head.version,
            size: 13,
            etag,
            uploaded: NOW,
            httpMetadata: { contentType: "text/plain" },
            customMetadata: { author: "fixture" },
        });
        expect(
            await client.files({
                ...bucket,
                prefix: "documents/",
                delimiter: "/",
                limit: 10,
            }),
        ).toEqual({ files: [head], delimitedPrefixes: ["documents/notes/"], cursor: null });

        // page through the prefix one file at a time
        const first = await client.files({ ...bucket, prefix: "documents/", limit: 1 });
        const nestedHead = await client.file({
            ...bucket,
            key: "documents/notes/first.txt",
        });
        expect(first).toEqual({ files: [nestedHead], delimitedPrefixes: [], cursor: first.cursor });
        expect(
            await client.files({
                ...bucket,
                prefix: "documents/",
                limit: 1,
                cursor: present(first.cursor, "cursor"),
            }),
        ).toEqual({ files: [head], delimitedPrefixes: [], cursor: null });

        // presign a download that signs its range
        const download = await client.open({
            mode: "read",
            ...bucket,
            key: "documents/readme.txt",
            range: "bytes=0-4",
        });
        expect(download).toEqual({
            url: `https://s3.test/files/documents/readme.txt?${PRESIGNED}&X-Amz-SignedHeaders=host%3Brange&X-Amz-Signature=114a717ac193df4b84e741eb694130e38bc85b62b54cfe4e7690fd54621a6009`,
            mode: "read",
            headers: { range: "bytes=0-4" },
            expiresAt: NOW + LEASE_LIFETIME * 1000,
        });

        // download the byte range with the file's headers
        const partial = await fixture.transfer(download);
        expect({
            status: partial.status,
            headers: Object.fromEntries(partial.headers),
            body: await partial.text(),
        }).toEqual({
            status: 206,
            headers: {
                "accept-ranges": "bytes",
                "content-length": "5",
                "content-range": "bytes 0-4/13",
                "content-type": "text/plain",
                etag: `"${etag}"`,
                "last-modified": new Date(NOW).toUTCString(),
                "x-amz-meta-author": "fixture",
                "x-amz-request-id": present(partial.headers.get("x-amz-request-id"), "id"),
            },
            body: "Hello",
        });

        // remove both files, after which heads and downloads find nothing
        expect(
            await client.remove({
                ...bucket,
                requestId: RequestId.create(),
                keys: ["documents/readme.txt", "documents/notes/first.txt"],
            }),
        ).toEqual({});
        expect(await client.file({ ...bucket, key: "documents/readme.txt" })).toBeNull();
        const missing = await fixture.transfer(download);
        expect({ status: missing.status, body: await missing.text() }).toEqual({
            status: 404,
            body: errorDocument(
                "NoSuchKey",
                "the specified key does not exist",
                "/files/documents/readme.txt",
                missing,
            ),
        });
    },
);

/** Refuse transfers with failed preconditions. */
test.for(TEST_DIALECTS)("refuse transfers whose preconditions fail on %s", async (dialect) => {
    await using fixture = await BucketFixture.open(
        present(databases.get(dialect), "dialect").database,
    );
    const { client, bucket } = fixture;
    const key = "report.txt";

    // store a file, then refuse to replace it where it must be absent
    const first = await client.open({
        mode: "write",
        ...bucket,
        key,
        size: 6,
        httpMetadata: {},
        customMetadata: {},
        ifNoneMatch: "*",
    });
    expect((await fixture.transfer(first, "stored")).status).toBe(200);
    const absent = await client.open({
        mode: "write",
        ...bucket,
        key,
        size: 7,
        httpMetadata: {},
        customMetadata: {},
        ifNoneMatch: "*",
    });
    const stale = await client.open({
        mode: "write",
        ...bucket,
        key,
        size: 7,
        httpMetadata: {},
        customMetadata: {},
        ifMatch: "other",
    });

    // refuse writes and reads with another entity tag
    const download = await client.open({ mode: "read", ...bucket, key, ifMatch: "other" });
    const refused = [
        await fixture.transfer(absent, "changed"),
        await fixture.transfer(stale, "changed"),
        await fixture.transfer(download),
    ];
    expect(
        await Promise.all(
            refused.map(async (response) => ({
                status: response.status,
                body: await response.text(),
            })),
        ),
    ).toEqual(
        refused.map((response) => ({
            status: 412,
            body: errorDocument(
                "PreconditionFailed",
                "at least one of the preconditions you specified did not hold",
                "/files/report.txt",
                response,
            ),
        })),
    );

    // read the file unchanged where its entity tag matches
    const head = present(await client.file({ ...bucket, key }), "head");
    const current = await fixture.transfer(
        await client.open({ mode: "read", ...bucket, key, ifMatch: head.etag }),
    );
    expect({ status: current.status, body: await current.text() }).toEqual({
        status: 200,
        body: "stored",
    });
});

/** Bind a customer key into presigned transfers, and refuse reading the file without it. */
test.for(TEST_DIALECTS)("transfer a file under a customer key on %s", async (dialect) => {
    await using fixture = await BucketFixture.open(
        present(databases.get(dialect), "dialect").database,
    );
    const { client, bucket } = fixture;
    const customerKey = new Uint8Array(32).fill(7).toBase64();
    const md5 = createHash("md5").update(Uint8Array.fromBase64(customerKey)).digest("base64");

    // presign an upload that signs the encryption headers and keeps them as headers
    const upload = await client.open({
        mode: "write",
        ...bucket,
        key: "sealed.txt",
        size: 6,
        httpMetadata: {},
        customMetadata: {},
        customerKey,
    });
    expect(upload).toEqual({
        url: `https://s3.test/files/sealed.txt?${PRESIGNED}&X-Amz-SignedHeaders=content-length%3Bhost%3Bx-amz-server-side-encryption-customer-algorithm%3Bx-amz-server-side-encryption-customer-key%3Bx-amz-server-side-encryption-customer-key-md5&X-Amz-Signature=332bfc23d5554bad68779cab8a9c9514964c6d5af5997ed84838b79f980867d4`,
        mode: "write",
        headers: {
            "content-length": "6",
            "x-amz-server-side-encryption-customer-algorithm": "AES256",
            "x-amz-server-side-encryption-customer-key": customerKey,
            "x-amz-server-side-encryption-customer-key-md5": md5,
        },
        expiresAt: NOW + LEASE_LIFETIME * 1000,
    });
    expect((await fixture.transfer(upload, "hidden")).status).toBe(200);

    // read it with the key, and refuse a read without it
    const sealed = await fixture.transfer(
        await client.open({ mode: "read", ...bucket, key: "sealed.txt", customerKey }),
    );
    const bare = await fixture.transfer(
        await client.open({ mode: "read", ...bucket, key: "sealed.txt" }),
    );
    expect([
        { status: sealed.status, body: await sealed.text() },
        { status: bare.status, body: await bare.text() },
    ]).toEqual([
        { status: 200, body: "hidden" },
        {
            status: 400,
            body: errorDocument(
                "InvalidRequest",
                "the file is encrypted with another customer key",
                "/files/sealed.txt",
                bare,
            ),
        },
    ]);
});

/** Report storage failures a request caused as bad requests, with the storage message. */
test.for(TEST_DIALECTS)(
    "refuse keys and cursors the bucket refuses as bad requests on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client, bucket } = fixture;

        // refuse a key with a NUL character and a cursor no listing issued
        await expect(client.file({ ...bucket, key: "a\u0000b" })).rejects.toEqual(
            new ServiceError("BAD_REQUEST", {
                message: "file keys require 1–1024 UTF-8 bytes without NUL",
            }),
        );
        await expect(client.files({ ...bucket, cursor: "unissued", limit: 10 })).rejects.toEqual(
            new ServiceError("BAD_REQUEST", { message: "invalid file listing cursor" }),
        );
    },
);

/** Refuse keys with `.` or `..` segments, which URL paths resolve away, and keep dotted names. */
test.for(TEST_DIALECTS)(
    "refuse file keys with dot segments and accept dotted names on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client, bucket } = fixture;
        const refusal = new ServiceError("BAD_REQUEST", {
            message: "invalid input: key: file keys refuse . and .. segments",
        });

        // refuse keys that climb out of their prefix or stay in place
        for (const key of ["../other/secret", "a/../b", "a/./b", "a/..", "."]) {
            await expect(
                client.open({
                    mode: "write",
                    ...bucket,
                    key,
                    size: 1,
                    httpMetadata: {},
                    customMetadata: {},
                }),
            ).rejects.toEqual(refusal);
            await expect(client.open({ mode: "read", ...bucket, key })).rejects.toEqual(refusal);
        }

        // store hidden and dotted names at their exact keys
        const keys = ["a/..b", "a/.hidden"];
        for (const key of keys) {
            const upload = await client.open({
                mode: "write",
                ...bucket,
                key,
                size: 1,
                httpMetadata: {},
                customMetadata: {},
            });
            expect((await fixture.transfer(upload, "x")).status).toBe(200);
        }

        // read and list both under their own keys
        const heads = await Promise.all(keys.map((key) => client.file({ ...bucket, key })));
        expect(heads).toEqual(
            keys.map((key, index) => ({
                key,
                version: present(aligned(heads, index), "head").version,
                size: 1,
                etag: createHash("md5").update("x").digest("hex"),
                uploaded: NOW,
                httpMetadata: {},
                customMetadata: {},
            })),
        );
        expect(await client.files({ ...bucket, limit: 10 })).toEqual({
            files: heads,
            delimitedPrefixes: [],
            cursor: null,
        });
    },
);

/** Refuse uploads, custom metadata and ranges beyond S3's bounds before presigning. */
test.for(TEST_DIALECTS)(
    "refuse uploads, metadata and ranges beyond their bounds on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client, bucket } = fixture;
        const file = { ...bucket, key: "large.bin", httpMetadata: {}, customMetadata: {} };

        // refuse an upload over 5 GiB, metadata over 2048 bytes and a range over 64 characters
        await expect(
            client.open({ mode: "write", ...file, size: 5 * 1024 ** 3 + 1 }),
        ).rejects.toEqual(
            new ServiceError("BAD_REQUEST", {
                message: "invalid input: size: too big: expected number to be <=5368709120",
            }),
        );
        await expect(
            client.open({
                mode: "write",
                ...file,
                size: 1,
                customMetadata: { note: "x".repeat(2045) },
            }),
        ).rejects.toEqual(
            new ServiceError("BAD_REQUEST", {
                message: "custom metadata is at most 2048 bytes",
            }),
        );
        await expect(
            client.open({
                mode: "read",
                ...bucket,
                key: "large.bin",
                range: `bytes=0-${"9".repeat(58)}`,
            }),
        ).rejects.toEqual(
            new ServiceError("BAD_REQUEST", {
                message: "invalid input: range: too big: expected string to have <=64 characters",
            }),
        );
    },
);

/** Decide each procedure on the bucket's permission, auditing refused uploads. */
test.for(TEST_DIALECTS)("refuse callers without the bucket permission on %s", async (dialect) => {
    await using fixture = await BucketFixture.open(
        present(databases.get(dialect), "dialect").database,
    );
    const { client, bucket } = fixture;
    const member = principal.user.reference("universe", fixture.userId);

    // refuse writing once neither write nor edit applies, while reading still does
    await fixture.withdraw("write");
    await fixture.withdraw("edit");
    await expect(
        client.open({
            mode: "write",
            ...bucket,
            key: "file",
            size: 1,
            httpMetadata: {},
            customMetadata: {},
        }),
    ).rejects.toEqual(
        new ServiceError("FORBIDDEN", { message: "permission denied: write", defined: true }),
    );
    expect(await client.files({ ...bucket, limit: 10 })).toEqual({
        files: [],
        delimitedPrefixes: [],
        cursor: null,
    });

    // audit the refused upload as a denial in the bucket's space
    const audits = await fixture.audits();
    const denial = aligned(audits, 0);
    expect(audits).toEqual([
        {
            method: invokeService.name,
            input: {},
            release: invokeService.package.version,
            execution: {
                id: denial.execution.id,
                requestId: denial.execution.requestId,
                category: "denial",
                context: {
                    caller: { type: "subject", subject: member },
                    package: spaceService.package,
                    scope: fixture.spaceId,
                    service: spaceService.name,
                },
                details: { authentication: "public" },
                outcome: {
                    kind: "denied",
                    error: { code: "FORBIDDEN", status: 403, message: "permission denied: write" },
                },
                targets: { procedure: { type: "procedure", id: "bucket.open" } },
                startedAt: NOW,
                finishedAt: NOW,
            },
        },
    ]);

    // refuse a bucket the space does not have
    const other = { ...bucket, id: schema.identifier("bucket").parse(`bucket-${v7()}`) };
    await expect(client.files({ ...other, limit: 10 })).rejects.toEqual(
        new ServiceError("NOT_FOUND", { message: `no bucket ${other.id}`, defined: true }),
    );

    // conceal the space from a caller outside the account
    fixture.caller = fixture.authenticate(member, [member]);
    await expect(client.file({ ...bucket, key: "file" })).rejects.toEqual(
        new ServiceError("NOT_FOUND", { message: `no scope ${fixture.spaceId}`, defined: true }),
    );
});

/** Conceal another space's bucket, which the member's role in its own space never reaches. */
test.for(TEST_DIALECTS)(
    "conceal a bucket of another space selected by its identifier on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client } = fixture;
        const neighbour = await fixture.neighbour();

        // conceal the other space from every method, while the member's own bucket answers
        const concealed = new ServiceError("NOT_FOUND", {
            message: `no scope ${neighbour.spaceId}`,
            defined: true,
        });
        await expect(client.files({ ...neighbour, limit: 10 })).rejects.toEqual(concealed);
        await expect(client.open({ mode: "read", ...neighbour, key: "secret" })).rejects.toEqual(
            concealed,
        );
        await expect(
            client.open({
                mode: "write",
                ...neighbour,
                key: "secret",
                size: 1,
                httpMetadata: {},
                customMetadata: {},
            }),
        ).rejects.toEqual(concealed);
        await expect(
            client.remove({
                ...neighbour,
                requestId: RequestId.create(),
                keys: ["secret"],
            }),
        ).rejects.toEqual(concealed);
        expect(await client.files({ ...fixture.bucket, limit: 10 })).toEqual({
            files: [],
            delimitedPrefixes: [],
            cursor: null,
        });
    },
);

/** Let an installation using the bucket reach its files, and refuse one that does not. */
test.for(TEST_DIALECTS)(
    "reach a bucket's files as an installation using it, and refuse another on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client, bucket, spaceId } = fixture;
        const using = principal.installation.reference(spaceId, await fixture.install("files"));
        const other = principal.installation.reference(
            spaceId,
            schema.identifier("installation").parse(`installation-${v7()}`),
        );

        // upload and list as the installation the bucket's capture relates as a consumer
        fixture.caller = fixture.authenticate(using, [using]);
        const upload = await client.open({
            mode: "write",
            ...bucket,
            key: "shared.txt",
            size: 6,
            httpMetadata: {},
            customMetadata: {},
        });
        expect((await fixture.transfer(upload, "shared")).status).toBe(200);
        const head = await client.file({ ...bucket, key: "shared.txt" });
        expect(await client.files({ ...bucket, limit: 10 })).toEqual({
            files: [head],
            delimitedPrefixes: [],
            cursor: null,
        });

        // conceal the space from an installation no capture relates to the bucket
        fixture.caller = fixture.authenticate(other, [other]);
        await expect(client.files({ ...bucket, limit: 10 })).rejects.toEqual(
            new ServiceError("NOT_FOUND", { message: `no scope ${spaceId}`, defined: true }),
        );
    },
);

/** Keep a bucket the system alone writes from its editors and consumers, who still read it. */
test.for(TEST_DIALECTS)(
    "refuse writes into a bucket the system alone writes to editors and consumers, who read on, on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client, bucket, spaceId } = fixture;
        await fixture.files.put("builds/manifest.json", "{}");
        const { uploadId } = await fixture.files.createMultipartUpload("builds/archive.tar");
        const consumer = principal.installation.reference(spaceId, await fixture.install("files"));

        // declare the bucket written by the system alone
        await fixture.database
            .update(bucketObject.bucket.table)
            .set({ spec: { write: "system" } })
            .where(eq(bucketObject.bucket.table.id, fixture.bucketId));

        // refuse every write of the editor, and a lease of the consumer
        const writes = (requestId: () => string) => [
            client.open({
                mode: "write",
                ...bucket,
                key: "builds/other.json",
                size: 2,
                httpMetadata: {},
                customMetadata: {},
            }),
            client.uploadPart({
                ...bucket,
                key: "builds/archive.tar",
                uploadId,
                partNumber: 1,
                size: 1,
            }),
            client.remove({ ...bucket, requestId: requestId(), keys: ["builds/manifest.json"] }),
            client.createUpload({
                ...bucket,
                requestId: requestId(),
                key: "builds/other.tar",
                httpMetadata: {},
                customMetadata: {},
            }),
            client.completeUpload({
                ...bucket,
                requestId: requestId(),
                key: "builds/archive.tar",
                uploadId,
                parts: [{ partNumber: 1, etag: "etag" }],
            }),
            client.abortUpload({
                ...bucket,
                requestId: requestId(),
                key: "builds/archive.tar",
                uploadId,
            }),
        ];
        const refusals = await Promise.all(
            writes(() => RequestId.create()).map((refused) =>
                refused.then(
                    () => "accepted",
                    (error: unknown) => error,
                ),
            ),
        );
        fixture.caller = fixture.authenticate(consumer, [consumer]);
        const leased = await client
            .open({
                mode: "write",
                ...bucket,
                key: "builds/other.json",
                size: 2,
                httpMetadata: {},
                customMetadata: {},
            })
            .catch((error: unknown) => error);

        // read the bucket's files as the consumer
        const download = await client.open({
            mode: "read",
            ...bucket,
            key: "builds/manifest.json",
        });
        const read = await fixture.transfer(download);
        const refused = new ServiceError("FORBIDDEN", {
            message: `the system alone writes ${fixture.bucketId}`,
            defined: true,
        });
        expect({ refusals, leased, read: [read.status, await read.text()] }).toEqual({
            refusals: Array.from({ length: 6 }, () => refused),
            leased: refused,
            read: [200, "{}"],
        });
    },
);

/** Refuse writes into a bucket a transfer fenced as retryable, presigned ones included, until the fence lifts. */
test.for(TEST_DIALECTS)(
    "refuse writes into a fenced bucket until its fence lifts, while reads go on, on %s",
    async (dialect) => {
        await using fixture = await BucketFixture.open(
            present(databases.get(dialect), "dialect").database,
        );
        const { client, bucket } = fixture;
        const file = {
            ...bucket,
            key: "notes/a.txt",
            size: 5,
            httpMetadata: {},
            customMetadata: {},
        };
        const lease = await client.open({ mode: "write", ...file });

        // refuse a lease, a removal and the earlier lease's transfer while fenced, and list on
        await fixture.files.fence();
        const leased = await client
            .open({ mode: "write", ...file })
            .catch((error: unknown) => error);
        const removed = await client
            .remove({ ...bucket, requestId: RequestId.create(), keys: ["notes/a.txt"] })
            .catch((error: unknown) => error);
        const transferred = await fixture.transfer(lease, "first");
        const listed = await client.files({ ...bucket, limit: 10 });

        // transfer the lease once the fence lifts
        await fixture.files.lift();
        const lifted = await fixture.transfer(lease, "first");
        const fenced = new ServiceError("UNAVAILABLE", {
            message: "bucket is fenced while a transfer copies it",
            status: 503,
            defined: true,
        });
        expect({
            leased,
            removed,
            transferred: [transferred.status, await transferred.text()],
            listed,
            lifted: lifted.status,
        }).toEqual({
            leased: fenced,
            removed: fenced,
            transferred: [
                503,
                errorDocument(
                    "ServiceUnavailable",
                    fenced.message,
                    "/files/notes/a.txt",
                    transferred,
                ),
            ],
            listed: { files: [], delimitedPrefixes: [], cursor: null },
            lifted: 200,
        });
    },
);

/** The S3 error document a response carries, with its request identifier. */
function errorDocument(
    code: string,
    message: string,
    resource: string,
    response: Response,
): string {
    const requestId = response.headers.get("x-amz-request-id");

    return `<?xml version="1.0" encoding="UTF-8"?><Error><Code>${code}</Code><Message>${message}</Message><Resource>${resource}</Resource><RequestId>${requestId}</RequestId></Error>`;
}
