import { Package } from "@destack/package";
import { space } from "@destack/account/object";
import { principal } from "@destack/access";
import { procedureInvoke } from "@destack/object/server";
import { eq } from "@destack/db";
import { nth, present, Identifier } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import stackPackage from "../../package.ts";
import * as bucketObject from "../../object/index.ts";

import { LEASE_LIFETIME_SECONDS } from "@destack/bucket/s3";
import { TestBucket, NOW, PRESIGNED } from "../../test/index.ts";
import { Simulation } from "@destack/environment/simulation";

/** The base64 MD5 of the customer key of 32 bytes of 7, as SSE-C sends it. */
const KEY_MD5 = "y4HAEFCYWuvAXWFTtA1Qpg==";

/** Open a bucket fixture over a spaces database in a world of its own, at the presigning time. */
async function openBucket(): Promise<TestBucket> {
    const world = new Simulation({ seed: 1, start: NOW }).environment("space");
    const storage = await TestBucket.database(world);
    onTestFinished(() => storage.close());

    return await TestBucket.open(storage.database, world);
}

/** Upload through a presigned transfer, list, head, download ranges and remove files. */
test("transfer files through presigned URLs and read their metadata", async () => {
    await using fixture = await openBucket();
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
        expiresAt: NOW + LEASE_LIFETIME_SECONDS * 1000,
    });

    // store the body through the transfer, answering its entity tag
    const etag = "3c5c1a70a9d2391707886435acfe01a6";
    const stored = await fixture.transfer(upload, "Hello, bucket");
    const version = present(stored.headers.get("x-amz-version-id"), "version");
    expect({ status: stored.status, headers: Object.fromEntries(stored.headers) }).toEqual({
        status: 200,
        headers: {
            etag: `"${etag}"`,
            "last-modified": new Date(NOW).toUTCString(),
            "x-amz-request-id": present(stored.headers.get("x-amz-request-id"), "id"),
            "x-amz-version-id": version,
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

    // read the stored metadata, and list it next to the nested prefix
    const head = present(await client.file({ ...bucket, key: "documents/readme.txt" }), "head");
    expect(head).toEqual({
        key: "documents/readme.txt",
        version,
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
    ).toEqual({
        files: [{ key: head.key, size: 13, etag, uploaded: NOW, storageClass: "Standard" }],
        delimitedPrefixes: ["documents/notes/"],
        cursor: null,
    });

    // page through the prefix one file at a time
    const first = await client.files({ ...bucket, prefix: "documents/", limit: 1 });
    const nestedEntry = {
        key: "documents/notes/first.txt",
        size: 5,
        etag: "4358b5009c67d0e31d7fbf1663fcd3bf",
        uploaded: NOW,
        storageClass: "Standard",
    };
    expect(first).toEqual({
        files: [nestedEntry],
        delimitedPrefixes: [],
        cursor: first.cursor,
    });
    expect(
        await client.files({
            ...bucket,
            prefix: "documents/",
            limit: 1,
            cursor: present(first.cursor, "cursor"),
        }),
    ).toEqual({
        files: [{ key: head.key, size: 13, etag, uploaded: NOW, storageClass: "Standard" }],
        delimitedPrefixes: [],
        cursor: null,
    });

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
        expiresAt: NOW + LEASE_LIFETIME_SECONDS * 1000,
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
            "x-amz-version-id": version,
        },
        body: "Hello",
    });

    // remove both files, after which heads and downloads find nothing
    expect(
        await client.remove({
            ...bucket,
            requestId: RequestId.create(fixture.environment),
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
});

/** Upload and read a file encrypted under a customer key, presigning the key's headers. */
test("transfer a file under a customer key", async () => {
    await using fixture = await openBucket();
    const { client, bucket } = fixture;
    const customerKey = new Uint8Array(32).fill(7).toBase64();
    const md5 = KEY_MD5;

    // presign an upload that signs the encryption headers and sends them as headers
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
        expiresAt: NOW + LEASE_LIFETIME_SECONDS * 1000,
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

/** Report storage failures and oversized keys as bad requests, with the storage message. */
test("refuse keys and cursors the bucket refuses, and keys over 1024 UTF-8 bytes, as bad requests", async () => {
    await using fixture = await openBucket();
    const { client, bucket } = fixture;

    // refuse a key with a NUL character and a cursor no listing issued
    expect(await refusal(client.file({ ...bucket, key: "a\u0000b" }))).toEqual([
        "BAD_REQUEST",
        "INVALID_ARGUMENT",
        "file keys require 1–1024 UTF-8 bytes without NUL",
    ]);
    expect(await refusal(client.files({ ...bucket, cursor: "unissued", limit: 10 }))).toEqual([
        "BAD_REQUEST",
        "INVALID_ARGUMENT",
        "invalid file listing cursor",
    ]);

    // refuse a lease on a key of 600 characters in 1200 UTF-8 bytes
    expect(await refusal(client.open({ mode: "read", ...bucket, key: "é".repeat(600) }))).toEqual([
        "BAD_REQUEST",
        "INVALID_ARGUMENT",
        "file keys require 1–1024 UTF-8 bytes without NUL",
    ]);
});

/** Refuse keys with `.` or `..` segments, which URL paths resolve away, and accept dotted names. */
test("refuse file keys with dot segments and accept dotted names", async () => {
    await using fixture = await openBucket();
    const { client, bucket } = fixture;
    const dotted = [
        "BAD_REQUEST",
        "INPUT_INVALID",
        "invalid input: key: file keys refuse . and .. segments",
    ];

    // refuse keys that climb out of their prefix or stay in place
    for (const key of ["../other/secret", "a/../b", "a/./b", "a/..", "."]) {
        const written = client.open({
            mode: "write",
            ...bucket,
            key,
            size: 1,
            httpMetadata: {},
            customMetadata: {},
        });
        const read = client.open({ mode: "read", ...bucket, key });
        expect([await refusal(written), await refusal(read)]).toEqual([dotted, dotted]);
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
            version: present(nth(heads, index), "head").version,
            size: 1,
            etag: "9dd4e461268c8034f5c8564e155c67a6",
            uploaded: NOW,
            httpMetadata: {},
            customMetadata: {},
        })),
    );
    expect(await client.files({ ...bucket, limit: 10 })).toEqual({
        files: keys.map((key) => ({
            key,
            size: 1,
            etag: "9dd4e461268c8034f5c8564e155c67a6",
            uploaded: NOW,
            storageClass: "Standard",
        })),
        delimitedPrefixes: [],
        cursor: null,
    });
});

/** Refuse uploads, custom metadata and ranges beyond S3's bounds before presigning. */
test("refuse uploads, metadata and ranges beyond their bounds", async () => {
    await using fixture = await openBucket();
    const { client, bucket } = fixture;
    const file = { ...bucket, key: "large.bin", httpMetadata: {}, customMetadata: {} };

    // refuse an upload over 5 GiB, metadata over 2048 bytes and a range over 64 characters
    expect(await refusal(client.open({ mode: "write", ...file, size: 5 * 1024 ** 3 + 1 }))).toEqual(
        [
            "BAD_REQUEST",
            "INPUT_INVALID",
            "invalid input: size: too big: expected number to be <=5368709120",
        ],
    );
    expect(
        await refusal(
            client.open({
                mode: "write",
                ...file,
                size: 1,
                customMetadata: { note: "x".repeat(2045) },
            }),
        ),
    ).toEqual(["BAD_REQUEST", "METADATA_TOO_LARGE", "custom metadata cannot exceed 2048 bytes"]);
    expect(
        await refusal(
            client.open({
                mode: "read",
                ...bucket,
                key: "large.bin",
                range: `bytes=0-${"9".repeat(58)}`,
            }),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "INPUT_INVALID",
        "invalid input: range: too big: expected string to have <=64 characters",
    ]);
});

/** Decide each procedure on the bucket's permission, auditing refused uploads. */
test("refuse callers without the bucket permission", async () => {
    await using fixture = await openBucket();
    const { client, bucket } = fixture;
    const member = principal.user.reference("universe", fixture.userId);

    // refuse writing once neither write nor edit applies, while reading still does
    await fixture.withdraw("write");
    await fixture.withdraw("edit");
    expect(
        await refusal(
            client.open({
                mode: "write",
                ...bucket,
                key: "file",
                size: 1,
                httpMetadata: {},
                customMetadata: {},
            }),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: write"]);
    expect(await client.files({ ...bucket, limit: 10 })).toEqual({
        files: [],
        delimitedPrefixes: [],
        cursor: null,
    });

    // audit the refused upload as a denial in the bucket's space
    const audits = await fixture.audits();
    const denial = nth(audits, 0);
    expect(audits).toEqual([
        {
            method: procedureInvoke.name,
            input: {},
            release: procedureInvoke.package.version,
            execution: {
                id: denial.execution.id,
                requestId: denial.execution.requestId,
                category: "denial",
                context: {
                    caller: { subject: member },
                    package: Package.of(stackPackage),
                    scope: fixture.spaceId,
                    traceId: denial.execution.context.traceId,
                },
                details: { authentication: "public" },
                outcome: {
                    kind: "denied",
                    error: {
                        code: "FORBIDDEN",
                        reason: "PERMISSION_DENIED",
                        domain: "@destack/access",
                        message: "permission denied: write",
                    },
                },
                target: { type: "procedure", id: "bucket.open" },
                startedAt: NOW,
                finishedAt: NOW,
            },
        },
    ]);

    // refuse a bucket the space does not have
    const other = { ...bucket, id: Identifier.create("bucket", fixture.environment) };
    expect(await refusal(client.files({ ...other, limit: 10 }))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `bucket ${other.id} not found`,
    ]);

    // conceal the space from a caller outside the account
    fixture.authentication = fixture.authenticate(member, [member]);
    expect(await refusal(client.file({ ...bucket, key: "file" }))).toEqual([
        "NOT_FOUND",
        "SCOPE_NOT_FOUND",
        `scope ${fixture.spaceId} not found`,
    ]);
});

/** Conceal another space's bucket, which the member's role in its own space never grants. */
test("conceal a bucket of another space selected by its identifier", async () => {
    await using fixture = await openBucket();
    const { client } = fixture;
    const neighbour = await fixture.neighbour();

    // conceal the other space from every method, while the member's own bucket answers
    const concealed = ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${neighbour.spaceId} not found`];
    const refused = [
        await refusal(client.files({ ...neighbour, limit: 10 })),
        await refusal(client.open({ mode: "read", ...neighbour, key: "secret" })),
        await refusal(
            client.open({
                mode: "write",
                ...neighbour,
                key: "secret",
                size: 1,
                httpMetadata: {},
                customMetadata: {},
            }),
        ),
        await refusal(
            client.remove({
                ...neighbour,
                requestId: RequestId.create(fixture.environment),
                keys: ["secret"],
            }),
        ),
    ];
    expect(refused).toEqual([concealed, concealed, concealed, concealed]);
    expect(await client.files({ ...fixture.bucket, limit: 10 })).toEqual({
        files: [],
        delimitedPrefixes: [],
        cursor: null,
    });
});

/** Let an installation using the bucket open its files, and refuse one that does not. */
test("open a bucket's files as an installation using it, and refuse another", async () => {
    await using fixture = await openBucket();
    const { client, bucket, spaceId } = fixture;
    const using = principal.installation.reference(spaceId, await fixture.install());
    const other = principal.installation.reference(
        spaceId,
        Identifier.create("installation", fixture.environment),
    );

    // upload and list as the installation the bucket's capture relates as a consumer
    fixture.authentication = fixture.authenticate(using, [using]);
    const upload = await client.open({
        mode: "write",
        ...bucket,
        key: "shared.txt",
        size: 6,
        httpMetadata: {},
        customMetadata: {},
    });
    expect((await fixture.transfer(upload, "shared")).status).toBe(200);
    expect(await client.files({ ...bucket, limit: 10 })).toEqual({
        files: [
            {
                key: "shared.txt",
                size: 6,
                etag: "9e81e7b963c71363e2fb3eefcfecfc0e",
                uploaded: NOW,
                storageClass: "Standard",
            },
        ],
        delimitedPrefixes: [],
        cursor: null,
    });

    // conceal the bucket from an installation no capture relates to it
    fixture.authentication = fixture.authenticate(other, [other]);
    expect(await refusal(client.files({ ...bucket, limit: 10 }))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `bucket ${bucket.id} not found`,
    ]);
});

/** Refuse editors' and consumers' writes to a bucket the space alone writes, and allow reads. */
test("refuse editors and consumers writes into a bucket the space alone writes", async () => {
    await using fixture = await openBucket();
    const { client, bucket, spaceId } = fixture;
    await fixture.files.put("builds/manifest.json", "{}");
    const { uploadId } = await fixture.files.createMultipartUpload("builds/archive.tar");
    const consumer = principal.installation.reference(spaceId, await fixture.install());

    // declare the bucket written by the system alone
    await fixture.database.transaction(
        async (transaction) =>
            await transaction
                .update(bucketObject.bucket.table)
                .set({ spec: { write: "package" } })
                .where(eq(bucketObject.bucket.table.id, fixture.bucketId)),
    );

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
        writes(() => RequestId.create(fixture.environment)).map((written) => refusal(written)),
    );
    fixture.authentication = fixture.authenticate(consumer, [consumer]);
    const leased = await refusal(
        client.open({
            mode: "write",
            ...bucket,
            key: "builds/other.json",
            size: 2,
            httpMetadata: {},
            customMetadata: {},
        }),
    );

    // read the bucket's files as the consumer
    const download = await client.open({
        mode: "read",
        ...bucket,
        key: "builds/manifest.json",
    });
    const read = await fixture.transfer(download);
    const refused = ["FORBIDDEN", "SPACE_ONLY", `only the space can write ${fixture.bucketId}`];
    expect({ refusals, leased, read: [read.status, await read.text()] }).toEqual({
        refusals: Array.from({ length: 6 }, () => refused),
        leased: refused,
        read: [200, "{}"],
    });
});

/** Refuse writes into a bucket a transfer fenced as retryable until the fence lifts. */
test("refuse writes into a fenced bucket until its fence lifts, while reads go on", async () => {
    await using fixture = await openBucket();
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
    const leased = await refusal(client.open({ mode: "write", ...file }));
    const removed = await refusal(
        client.remove({
            ...bucket,
            requestId: RequestId.create(fixture.environment),
            keys: ["notes/a.txt"],
        }),
    );
    const transferred = await fixture.transfer(lease, "first");
    const listed = await client.files({ ...bucket, limit: 10 });

    // transfer the lease once the fence lifts
    await fixture.files.lift();
    const lifted = await fixture.transfer(lease, "first");
    const fencedMessage = "bucket is fenced while a transfer copies it";
    const fenced = ["SERVICE_UNAVAILABLE", "UNAVAILABLE", fencedMessage];
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
            errorDocument("ServiceUnavailable", fencedMessage, "/files/notes/a.txt", transferred),
        ],
        listed: { files: [], delimitedPrefixes: [], cursor: null },
        lifted: 200,
    });
});

/** The S3 error document of a response, with its request identifier. */
function errorDocument(
    code: string,
    message: string,
    resource: string,
    response: Response,
): string {
    const requestId = response.headers.get("x-amz-request-id");

    return `<?xml version="1.0" encoding="UTF-8"?><Error><Code>${code}</Code><Message>${message}</Message><Resource>${resource}</Resource><RequestId>${requestId}</RequestId></Error>`;
}

/** Refuse writes once the space's storage entitlement is exceeded, allowing reads and removals. */
test("refuse writes into a space whose storage exceeds its entitlement, allowing reads and removals", async () => {
    await using fixture = await openBucket();
    const { client, bucket } = fixture;

    // cap the space's storage, as its machine records the account's blocked entitlement
    const capped = {
        status: "false",
        observedGeneration: 1,
        reason: "Blocked",
        message: "storage is at its limit of 1000",
        lastTransitionAt: NOW,
    } as const;
    await fixture.database.transaction(
        async (transaction) =>
            await transaction
                .update(space.table)
                .set({ conditions: { storage: capped } })
                .where(eq(space.table.id, fixture.spaceId)),
    );

    // refuse an upload and a multipart upload, and read and remove as before
    const write = {
        mode: "write",
        ...bucket,
        key: "a.txt",
        size: 1,
        httpMetadata: {},
        customMetadata: {},
    } as const;
    const refused = await refusal(client.open(write));
    const multipart = await refusal(
        client.createUpload({
            ...bucket,
            requestId: RequestId.create(fixture.environment),
            key: "b.bin",
            httpMetadata: {},
            customMetadata: {},
        }),
    );
    const listed = await client.files({ ...bucket, limit: 10 });
    const removed = await client.remove({
        ...bucket,
        requestId: RequestId.create(fixture.environment),
        keys: ["a.txt"],
    });

    const message = `space ${fixture.spaceId} exceeds its entitlement: storage is at its limit of 1000`;
    expect({ refused, multipart, listed: listed.files, removed }).toEqual({
        refused: ["PAYMENT_REQUIRED", "ENTITLEMENT_EXCEEDED", message],
        multipart: ["PAYMENT_REQUIRED", "ENTITLEMENT_EXCEEDED", message],
        listed: [],
        removed,
    });
});
