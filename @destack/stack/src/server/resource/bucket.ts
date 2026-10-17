import type { MethodContext, MethodContextOf, ResultOf } from "@destack/object";
import { Caller } from "@destack/access";
import { schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { Space } from "@destack/account/object";
import { CallError } from "@destack/runtime/error";
import {
    BucketCustomMetadata,
    BucketHttpMetadata,
    type BucketFile,
    BucketSpec,
    MAX_CUSTOM_METADATA_BYTES,
} from "@destack/bucket";
import { bucket, type FileMetadata } from "../../object/index.ts";
import {
    type BucketHost,
    BucketLease,
    type BucketReference,
    customerKeyHeaders,
} from "@destack/bucket/s3";

/** Implement buckets: their files' metadata, and presigned leases on their bodies. */
export function implementBucket(host: BucketHost) {
    return bucket.implement({
        methods: {
            files: (call) => listFiles(host, call),
            file: (call) => headFile(host, call),
            open: { authorize: authorizeOpen, handle: (context) => openLease(host, context) },
            remove: {
                authorize: async (context) => requireWriter(context),
                handle: (context) => removeFiles(host, context),
            },
            createUpload: {
                authorize: async (context) => {
                    requireWriter(context);
                    await requireStorage(context);
                },
                handle: (context) => createUpload(host, context),
            },
            uploadPart: {
                authorize: async (context) => {
                    requireWriter(context);
                    await requireStorage(context);
                },
                handle: (context) => uploadPart(host, context),
            },
            completeUpload: {
                authorize: async (context) => {
                    requireWriter(context);
                    await requireStorage(context);
                },
                handle: (context) => completeUpload(host, context),
            },
            abortUpload: {
                authorize: async (context) => requireWriter(context),
                handle: (context) => abortUpload(host, context),
            },
        },
    });
}

/** List a page of a bucket's files. */
function listFiles(
    host: BucketHost,
    context: MethodContextOf<typeof bucket, "files">,
): Promise<ResultOf<typeof bucket, "files">> {
    return withBucket(host, context, async (opened) => {
        // list a page of files
        const input = context.input;
        const page = await opened.list({
            ...(input.prefix === undefined ? {} : { prefix: input.prefix }),
            ...(input.delimiter === undefined ? {} : { delimiter: input.delimiter }),
            ...(input.cursor === undefined ? {} : { cursor: input.cursor }),
            limit: input.limit,
        });

        return {
            files: page.files.map((entry) => ({ ...entry, uploaded: entry.uploaded.getTime() })),
            delimitedPrefixes: page.delimitedPrefixes,
            cursor: page.truncated ? page.cursor : null,
        };
    });
}

/** Read one file's metadata, null for a missing file. */
function headFile(
    host: BucketHost,
    context: MethodContextOf<typeof bucket, "file">,
): Promise<ResultOf<typeof bucket, "file">> {
    return withBucket(host, context, async (opened) => {
        const file = await opened.head(context.input.key);

        return file === null ? null : describe(file);
    });
}

/** Require the right to upload for a write. */
async function authorizeOpen(context: MethodContextOf<typeof bucket, "open">): Promise<void> {
    if (context.input.mode === "write") {
        await context.require(bucket.permission("write"), context.reference());
        requireWriter(context);
        await requireStorage(context);
    }
}

/** Presign a lease reading or writing one file's body. */
function openLease(
    host: BucketHost,
    context: MethodContextOf<typeof bucket, "open">,
): Promise<ResultOf<typeof bucket, "open">> {
    // bind a read's range and entity tag
    const input = context.input;
    const headers = new Headers();
    if (input.mode === "read") {
        if (input.range !== undefined) {
            headers.set("range", input.range);
        }
    }
    // bind a write's body length, stored metadata and replacement precondition
    else {
        const customMetadata = input.customMetadata ?? {};
        requireMetadata(customMetadata);
        headers.set("content-length", String(input.size));
        BucketHttpMetadata.write(httpMetadata(input.httpMetadata ?? {}), headers);
        for (const [name, value] of Object.entries(customMetadata)) {
            headers.set(`x-amz-meta-${name}`, value);
        }
        if (input.ifNoneMatch !== undefined) {
            headers.set("if-none-match", input.ifNoneMatch);
        }
    }

    // bind the entity tag the file must have, and the customer key
    if (input.ifMatch !== undefined) {
        headers.set("if-match", input.ifMatch);
    }
    setCustomerKey(headers, input.customerKey);

    return BucketLease.presign(
        host,
        reference(context),
        { mode: input.mode, key: input.key, headers },
        context.now,
    );
}

/** Delete some files of a bucket. */
function removeFiles(
    host: BucketHost,
    context: MethodContextOf<typeof bucket, "remove">,
): Promise<ResultOf<typeof bucket, "remove">> {
    return withBucket(host, context, async (opened) => {
        await opened.delete(context.input.keys);

        return {};
    });
}

/** Create a multipart upload with the metadata its file receives. */
function createUpload(
    host: BucketHost,
    context: MethodContextOf<typeof bucket, "createUpload">,
): Promise<ResultOf<typeof bucket, "createUpload">> {
    return withBucket(host, context, async (opened) => {
        // create the upload with the metadata its file receives
        const input = context.input;
        requireMetadata(input.customMetadata);
        const upload = await opened.createMultipartUpload(input.key, {
            httpMetadata: httpMetadata(input.httpMetadata),
            customMetadata: input.customMetadata,
            ...(input.customerKey === undefined
                ? {}
                : { ssecKey: Uint8Array.fromBase64(input.customerKey).buffer }),
        });

        return { uploadId: upload.uploadId, key: upload.key };
    });
}

/** Presign a lease writing one part of a multipart upload. */
function uploadPart(
    host: BucketHost,
    context: MethodContextOf<typeof bucket, "uploadPart">,
): Promise<ResultOf<typeof bucket, "uploadPart">> {
    // bind the part's length and number
    const input = context.input;
    const headers = new Headers({ "content-length": String(input.size) });
    setCustomerKey(headers, input.customerKey);

    return BucketLease.presign(
        host,
        reference(context),
        {
            mode: "write",
            key: input.key,
            headers,
            query: { partNumber: String(input.partNumber), uploadId: input.uploadId },
        },
        context.now,
    );
}

/** Complete a multipart upload from its parts. */
function completeUpload(
    host: BucketHost,
    context: MethodContextOf<typeof bucket, "completeUpload">,
): Promise<ResultOf<typeof bucket, "completeUpload">> {
    return withBucket(host, context, async (opened) => {
        // assemble the parts by their entity tags without quotes
        const input = context.input;
        const upload = opened.resumeMultipartUpload(input.key, input.uploadId);
        const parts = input.parts.map((part) => ({
            partNumber: part.partNumber,
            etag: part.etag.replace(/^"(.*)"$/u, "$1"),
        }));

        return describe(await upload.complete(parts));
    });
}

/** Abort a multipart upload. */
function abortUpload(
    host: BucketHost,
    context: MethodContextOf<typeof bucket, "abortUpload">,
): Promise<ResultOf<typeof bucket, "abortUpload">> {
    return withBucket(host, context, async (opened) => {
        const input = context.input;
        await opened.resumeMultipartUpload(input.key, input.uploadId).abort();

        return {};
    });
}

/** A bucket method's context on a loaded bucket. */
type BucketContext = MethodContext<{ readonly table: (typeof bucket)["table"] }, unknown>;

/** Read the bucket a call targets. */
function reference(context: BucketContext): BucketReference {
    return {
        scope: schema.identifier("space").parse(context.scope),
        bucketId: schema.identifier("bucket").parse(context.target.id),
    };
}

/** Refuse a write into a bucket its package alone writes, unless the package calls as itself. */
function requireWriter(context: BucketContext): void {
    // refuse a write into a package's bucket by any caller but the package itself
    const target = context.target;
    const { subject } = context.caller;
    const isItself = subject !== undefined && Subject.same(subject, Caller.principal(context.self));
    if (BucketSpec.parse(target.spec).write === "package" && !isItself) {
        throw new CallError("FORBIDDEN", { message: `the space alone writes ${target.id}` });
    }
}

/** Refuse a write into a space whose storage entitlement is exceeded. */
async function requireStorage(context: BucketContext): Promise<void> {
    Space.requireWithin(await Space.read(context, context.scope), "storage");
}

/** Work on the bucket a call targets. */
async function withBucket<Value>(
    host: BucketHost,
    context: BucketContext,
    work: (opened: Awaited<ReturnType<BucketHost["open"]>>) => Promise<Value>,
): Promise<Value> {
    return work(await host.open(reference(context)));
}

/** Describe a file's metadata for callers, with times in UTC milliseconds. */
function describe(file: BucketFile): FileMetadata {
    const { cacheExpiry, ...headers } = file.httpMetadata;

    return {
        key: file.key,
        version: file.version,
        size: file.size,
        etag: file.etag,
        uploaded: file.uploaded.getTime(),
        httpMetadata:
            cacheExpiry === undefined
                ? headers
                : { ...headers, cacheExpiry: cacheExpiry.getTime() },
        customMetadata: file.customMetadata,
    };
}

/** Read HTTP metadata from its wire form, with the expiry in UTC milliseconds. */
function httpMetadata(metadata: FileMetadata["httpMetadata"]): BucketFile["httpMetadata"] {
    const { cacheExpiry, ...headers } = metadata;

    return cacheExpiry === undefined ? headers : { ...headers, cacheExpiry: new Date(cacheExpiry) };
}

/** Refuse custom metadata beyond S3's limit, counting names and values in UTF-8 bytes. */
function requireMetadata(metadata: Readonly<Record<string, string>>): void {
    if (BucketCustomMetadata.bytes(metadata) > MAX_CUSTOM_METADATA_BYTES) {
        throw new CallError("BAD_REQUEST", {
            message: `custom metadata is at most ${MAX_CUSTOM_METADATA_BYTES} bytes`,
        });
    }
}

/** Bind a customer key as SSE-C headers, when the caller sent one. */
function setCustomerKey(headers: Headers, customerKey: string | undefined): void {
    if (customerKey !== undefined) {
        for (const [name, value] of Object.entries(customerKeyHeaders(customerKey))) {
            headers.set(name, value);
        }
    }
}
