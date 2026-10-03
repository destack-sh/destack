import { present } from "@destack/schema";
import {
    CopyObjectCommand,
    DeleteObjectCommand,
    DeleteObjectsCommand,
    GetObjectCommand,
    HeadObjectCommand,
    ListObjectsV2Command,
    PutObjectCommand,
} from "@aws-sdk/client-s3";
import { createHash } from "node:crypto";
import { expect, test } from "@destack/test";
import { failure, S3Fixture } from "./test/fixture.ts";

test("store, read and delete objects with metadata through the AWS SDK", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();

    // store an object with HTTP metadata, user metadata and a storage class
    const stored = await client.send(
        new PutObjectCommand({
            Bucket: "files",
            Key: "documents/readme.txt",
            Body: "Hello, bucket",
            ContentType: "text/plain",
            CacheControl: "max-age=60",
            Metadata: { author: "alice" },
            StorageClass: "STANDARD_IA",
        }),
    );
    const file = present(
        await present(fixture.buckets.get("files"), "files").head("documents/readme.txt"),
        "file",
    );
    expect(stored.ETag).toBe(file.httpEtag);

    // read its metadata and its body
    const head = await client.send(
        new HeadObjectCommand({ Bucket: "files", Key: "documents/readme.txt" }),
    );
    expect({
        size: head.ContentLength,
        type: head.ContentType,
        cache: head.CacheControl,
        metadata: head.Metadata,
        storageClass: head.StorageClass,
        etag: head.ETag,
        modified: head.LastModified?.getTime(),
    }).toEqual({
        size: 13,
        type: "text/plain",
        cache: "max-age=60",
        metadata: { author: "alice" },
        storageClass: "STANDARD_IA",
        etag: file.httpEtag,
        modified: Math.floor(file.uploaded.getTime() / 1000) * 1000,
    });
    const object = await client.send(
        new GetObjectCommand({ Bucket: "files", Key: "documents/readme.txt" }),
    );
    expect(await present(object.Body, "Body").transformToString()).toBe("Hello, bucket");

    // delete it, after which reads find no key
    await client.send(new DeleteObjectCommand({ Bucket: "files", Key: "documents/readme.txt" }));
    expect(
        await failure(
            client.send(new GetObjectCommand({ Bucket: "files", Key: "documents/readme.txt" })),
        ),
    ).toEqual({
        name: "NoSuchKey",
        status: 404,
    });
    expect(
        await failure(
            client.send(new HeadObjectCommand({ Bucket: "files", Key: "documents/readme.txt" })),
        ),
    ).toEqual({
        name: "NotFound",
        status: 404,
    });
    expect(
        await failure(client.send(new GetObjectCommand({ Bucket: "missing", Key: "file" }))),
    ).toEqual({
        name: "NoSuchBucket",
        status: 404,
    });
});

test("apply conditional headers and byte ranges through the AWS SDK", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();
    const { ETag: etag } = await client.send(
        new PutObjectCommand({ Bucket: "files", Key: "report.txt", Body: "abcdef" }),
    );
    const get = (options: Partial<ConstructorParameters<typeof GetObjectCommand>[0]>) =>
        client.send(new GetObjectCommand({ Bucket: "files", Key: "report.txt", ...options }));

    // answer failed conditions with 412 or 304 as S3 does
    const later = new Date(Date.now() + 60_000);
    const earlier = new Date(Date.now() - 60_000);
    expect(await failure(get({ IfMatch: '"other"' }))).toEqual({
        name: "PreconditionFailed",
        status: 412,
    });
    expect(await failure(get({ IfUnmodifiedSince: earlier }))).toEqual({
        name: "PreconditionFailed",
        status: 412,
    });
    expect(await failure(get({ IfNoneMatch: etag }))).toEqual({ name: "Unknown", status: 304 });
    expect(await failure(get({ IfModifiedSince: later }))).toEqual({
        name: "Unknown",
        status: 304,
    });
    expect(
        await present(
            (await get({ IfMatch: etag, IfUnmodifiedSince: earlier })).Body,
            "Body",
        ).transformToString(),
    ).toBe("abcdef");

    // refuse conditional writes whose preconditions fail
    expect(
        await failure(
            client.send(
                new PutObjectCommand({
                    Bucket: "files",
                    Key: "report.txt",
                    Body: "x",
                    IfNoneMatch: "*",
                }),
            ),
        ),
    ).toEqual({ name: "PreconditionFailed", status: 412 });
    expect(
        await failure(
            client.send(
                new PutObjectCommand({
                    Bucket: "files",
                    Key: "absent.txt",
                    Body: "x",
                    IfMatch: etag,
                }),
            ),
        ),
    ).toEqual({ name: "NoSuchKey", status: 404 });

    // read ranges with 206, whole files for malformed ranges, and 416 past the end
    const ranged = await get({ Range: "bytes=1-3" });
    expect({
        range: ranged.ContentRange,
        length: ranged.ContentLength,
        body: await present(ranged.Body, "Body").transformToString(),
    }).toEqual({
        range: "bytes 1-3/6",
        length: 3,
        body: "bcd",
    });
    expect(await present((await get({ Range: "bytes=-2" })).Body, "Body").transformToString()).toBe(
        "ef",
    );
    expect(await present((await get({ Range: "bytes=4-" })).Body, "Body").transformToString()).toBe(
        "ef",
    );
    expect(
        await present((await get({ Range: "bytes=1-2,4-5" })).Body, "Body").transformToString(),
    ).toBe("abcdef");
    expect(await failure(get({ Range: "bytes=6-" }))).toEqual({
        name: "InvalidRange",
        status: 416,
    });
});

test("copy objects within and between buckets through the AWS SDK", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();
    const { ETag: etag } = await client.send(
        new PutObjectCommand({
            Bucket: "files",
            Key: "source.txt",
            Body: "copied",
            ContentType: "text/plain",
            Metadata: { author: "alice" },
        }),
    );

    // copy with the source's metadata, then with replaced metadata into another bucket
    const copy = await client.send(
        new CopyObjectCommand({
            Bucket: "files",
            Key: "copy.txt",
            CopySource: "files/source.txt",
            CopySourceIfMatch: etag,
        }),
    );
    expect(present(copy.CopyObjectResult, "CopyObjectResult").ETag).toBe(etag);
    const archived = await client.send(
        new CopyObjectCommand({
            Bucket: "archive",
            Key: "copy.txt",
            CopySource: "/files/source.txt",
            MetadataDirective: "REPLACE",
            ContentType: "text/markdown",
            Metadata: { reviewed: "yes" },
        }),
    );
    expect(present(archived.CopyObjectResult, "CopyObjectResult").ETag).toBe(etag);
    const heads = await Promise.all(
        [
            ["files", "copy.txt"],
            ["archive", "copy.txt"],
        ].map(async ([bucket, key]) => {
            const head = await client.send(new HeadObjectCommand({ Bucket: bucket, Key: key }));

            return { type: head.ContentType, metadata: head.Metadata };
        }),
    );
    expect(heads).toEqual([
        { type: "text/plain", metadata: { author: "alice" } },
        { type: "text/markdown", metadata: { reviewed: "yes" } },
    ]);

    // refuse failed source conditions, missing sources and copies that change nothing
    const copyTo = (options: Partial<ConstructorParameters<typeof CopyObjectCommand>[0]>) =>
        client.send(
            new CopyObjectCommand({
                Bucket: "files",
                Key: "copy.txt",
                CopySource: "files/source.txt",
                ...options,
            }),
        );
    expect(await failure(copyTo({ CopySourceIfNoneMatch: etag }))).toEqual({
        name: "PreconditionFailed",
        status: 412,
    });
    expect(await failure(copyTo({ CopySource: "files/missing.txt" }))).toEqual({
        name: "NoSuchKey",
        status: 404,
    });
    expect(await failure(copyTo({ Key: "source.txt" }))).toEqual({
        name: "InvalidRequest",
        status: 400,
    });
});

test("list objects by prefix, delimiter and continuation through the AWS SDK", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();
    for (const key of [
        "photos/2024/a.jpg",
        "photos/2024/b.jpg",
        "photos/2025/c.jpg",
        "photos/index.html",
        "readme.txt",
    ]) {
        await client.send(new PutObjectCommand({ Bucket: "files", Key: key, Body: key }));
    }
    const list = (options: Partial<ConstructorParameters<typeof ListObjectsV2Command>[0]>) =>
        client.send(new ListObjectsV2Command({ Bucket: "files", ...options }));

    // group keys below a prefix at the delimiter, one page at a time
    const pages = [];
    let token: string | undefined;
    do {
        const page = await list({
            Prefix: "photos/",
            Delimiter: "/",
            MaxKeys: 2,
            ContinuationToken: token,
        });
        pages.push({
            keys: page.Contents?.map((entry) => entry.Key) ?? [],
            prefixes: page.CommonPrefixes?.map((entry) => entry.Prefix) ?? [],
            count: page.KeyCount,
            isTruncated: page.IsTruncated,
        });
        token = page.NextContinuationToken;
    } while (token !== undefined);
    expect(pages).toEqual([
        { keys: [], prefixes: ["photos/2024/", "photos/2025/"], count: 2, isTruncated: true },
        { keys: ["photos/index.html"], prefixes: [], count: 1, isTruncated: false },
    ]);

    // list after a key, with the size, entity tag and storage class of each file
    const after = await list({ StartAfter: "photos/index.html" });
    const file = present(
        await present(fixture.buckets.get("files"), "files").head("readme.txt"),
        "readme.txt",
    );
    expect(after.Contents).toEqual([
        {
            Key: "readme.txt",
            LastModified: new Date(file.uploaded.toISOString()),
            ETag: file.httpEtag,
            Size: 10,
            StorageClass: "STANDARD",
        },
    ]);

    // delete several objects at once, reporting keys it refuses
    const deleted = await client.send(
        new DeleteObjectsCommand({
            Bucket: "files",
            Delete: {
                Objects: [
                    { Key: "photos/2024/a.jpg" },
                    { Key: "readme.txt" },
                    { Key: "missing" },
                    { Key: "photos/2024/b.jpg", VersionId: "1" },
                ],
            },
        }),
    );
    expect({ deleted: deleted.Deleted, errors: deleted.Errors }).toEqual({
        deleted: [{ Key: "photos/2024/a.jpg" }, { Key: "readme.txt" }, { Key: "missing" }],
        errors: [
            {
                Key: "photos/2024/b.jpg",
                Code: "NotImplemented",
                Message: "deleting versions is not supported",
            },
        ],
    });
    const remaining = await list({});
    expect(remaining.Contents?.map((entry) => entry.Key)).toEqual([
        "photos/2024/b.jpg",
        "photos/2025/c.jpg",
        "photos/index.html",
    ]);
});

test("match If-Match entity-tag lists on reads and writes through the AWS SDK", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();
    const { ETag: etag } = await client.send(
        new PutObjectCommand({ Bucket: "files", Key: "report.txt", Body: "abcdef" }),
    );
    const get = (ifMatch: string) =>
        client.send(new GetObjectCommand({ Bucket: "files", Key: "report.txt", IfMatch: ifMatch }));
    const put = (ifMatch: string, body: string) =>
        client.send(
            new PutObjectCommand({
                Bucket: "files",
                Key: "report.txt",
                Body: body,
                IfMatch: ifMatch,
            }),
        );

    // read and write when the list includes the current tag
    expect(await present((await get(`"other", ${etag}`)).Body, "Body").transformToString()).toBe(
        "abcdef",
    );
    const stored = await put(`${etag}, "other"`, "ghijkl");
    const file = present(
        await present(fixture.buckets.get("files"), "files").head("report.txt"),
        "report.txt",
    );
    expect(stored.ETag).toBe(file.httpEtag);

    // refuse reads and writes when the list names only other tags
    expect(await failure(get(`"other", ${etag}`))).toEqual({
        name: "PreconditionFailed",
        status: 412,
    });
    expect(await failure(put(`"other", "another"`, "mnopqr"))).toEqual({
        name: "PreconditionFailed",
        status: 412,
    });
    expect(
        await present(
            await present(fixture.buckets.get("files"), "files").get("report.txt"),
            "report.txt",
        ).text(),
    ).toBe("ghijkl");
});

test("refuse a write whose If-Match list names only an older version, storing nothing", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();
    const put = (body: string, ifMatch?: string) =>
        client.send(
            new PutObjectCommand({
                Bucket: "files",
                Key: "report.txt",
                Body: body,
                IfMatch: ifMatch,
            }),
        );

    // replace the file after reading its first tag
    const { ETag: older } = await put("first");
    await put("second");

    // refuse the write conditioned on the older tag and keep the second version
    expect(await failure(put("third", `${older}, "other"`))).toEqual({
        name: "PreconditionFailed",
        status: 412,
    });
    expect(
        await present(
            await present(fixture.buckets.get("files"), "files").get("report.txt"),
            "report.txt",
        ).text(),
    ).toBe("second");
});

test("compare weak If-None-Match tags weakly on reads and refuse them on writes", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();
    const { ETag: etag } = await client.send(
        new PutObjectCommand({ Bucket: "files", Key: "report.txt", Body: "abcdef" }),
    );

    // answer a read with 304 when the weak tag matches the current file
    expect(
        await failure(
            client.send(
                new GetObjectCommand({
                    Bucket: "files",
                    Key: "report.txt",
                    IfNoneMatch: `W/${etag}`,
                }),
            ),
        ),
    ).toEqual({ name: "Unknown", status: 304 });

    // refuse a write conditioned on a tag, as S3 supports only the asterisk, and store nothing
    expect(
        await failure(
            client.send(
                new PutObjectCommand({
                    Bucket: "files",
                    Key: "report.txt",
                    Body: "ghijkl",
                    IfNoneMatch: `W/${etag}`,
                }),
            ),
        ),
    ).toEqual({ name: "NotImplemented", status: 501 });
    expect(
        await present(
            await present(fixture.buckets.get("files"), "files").get("report.txt"),
            "report.txt",
        ).text(),
    ).toBe("abcdef");
});

test("match copy-source entity-tag lists against the current source through the AWS SDK", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();
    const put = (body: string) =>
        client.send(new PutObjectCommand({ Bucket: "files", Key: "source.txt", Body: body }));
    const copyTo = (
        key: string,
        options: Partial<ConstructorParameters<typeof CopyObjectCommand>[0]>,
    ) =>
        client.send(
            new CopyObjectCommand({
                Bucket: "files",
                Key: key,
                CopySource: "files/source.txt",
                ...options,
            }),
        );

    // copy when the list includes the current tag
    const { ETag: older } = await put("first");
    const copy = await copyTo("copy.txt", { CopySourceIfMatch: `"other", ${older}` });
    expect(present(copy.CopyObjectResult, "CopyObjectResult").ETag).toBe(older);

    // refuse lists without the current tag, weak matches of it, and tags of an older version
    const { ETag: etag } = await put("second");
    const failures = [
        await failure(copyTo("missed.txt", { CopySourceIfMatch: `"other", "another"` })),
        await failure(copyTo("weak.txt", { CopySourceIfNoneMatch: `W/${etag}` })),
        await failure(copyTo("stale.txt", { CopySourceIfMatch: `${older}, "other"` })),
    ];
    expect(failures).toEqual([
        { name: "PreconditionFailed", status: 412 },
        { name: "PreconditionFailed", status: 412 },
        { name: "PreconditionFailed", status: 412 },
    ]);

    // store none of the refused copies
    const listed = await present(fixture.buckets.get("files"), "files").list();
    expect(listed.files.map((file) => file.key)).toEqual(["copy.txt", "source.txt"]);
});

test("store and read objects under customer keys through the AWS SDK", async () => {
    await using fixture = await S3Fixture.open();
    const client = fixture.client();
    const key = new Uint8Array(32).fill(7);
    const customer = {
        SSECustomerAlgorithm: "AES256",
        SSECustomerKey: key.toBase64(),
        SSECustomerKeyMD5: createHash("md5").update(key).digest("base64"),
    };

    // store an object under the key, which its metadata names
    await client.send(
        new PutObjectCommand({ Bucket: "files", Key: "sealed.txt", Body: "hidden", ...customer }),
    );
    const head = await client.send(new HeadObjectCommand({ Bucket: "files", Key: "sealed.txt" }));
    expect([head.SSECustomerAlgorithm, head.SSECustomerKeyMD5]).toEqual([
        "AES256",
        customer.SSECustomerKeyMD5,
    ]);

    // read it with the key, and refuse reading it without
    const read = await client.send(
        new GetObjectCommand({ Bucket: "files", Key: "sealed.txt", ...customer }),
    );
    expect(await present(read.Body, "Body").transformToString()).toBe("hidden");
    expect(
        await failure(client.send(new GetObjectCommand({ Bucket: "files", Key: "sealed.txt" }))),
    ).toEqual({ name: "InvalidRequest", status: 400 });
});
