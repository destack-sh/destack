import type { Call, CallOf, PreparedCallOf, ResultOf } from "@destack/object";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { BucketHttpMetadata, type BucketFile } from "../bucket/index.ts";
import { BucketError } from "../error/index.ts";
import type { Lease, LeaseMode } from "@destack/resource";
import { BucketSpec } from "../declare/bucket.ts";
import { bucket, type FileMetadata } from "../object/index.ts";
import {
    type BucketHost,
    type BucketReference,
    customerKeyHeaders,
    S3Location,
    SignatureV4,
} from "../s3/index.ts";

/** How long a presigned lease stays valid in seconds: fifteen minutes, the AWS SDK's default. */
export const LEASE_LIFETIME = 15 * 60;

/** The most bytes of custom metadata names and values together, S3's 2 KB limit. */
const MAX_METADATA_BYTES = 2048;

/** The HTTP method a lease of each mode signs: GET reads a body, PUT writes one. */
const LEASE_METHODS: Readonly<Record<LeaseMode, "GET" | "PUT">> = { read: "GET", write: "PUT" };

/** One S3 request a presigned lease grants. */
interface LeaseRequest {
    /** What the lease lets its holder do. */
    readonly mode: LeaseMode;
    /** The file key. */
    readonly key: string;
    /** The headers the signature binds. */
    readonly headers: Headers;
    /** The query parameters selecting a multipart part. */
    readonly query?: Record<string, string>;
}

/** Serve the files of a host's buckets: their metadata, and presigned leases on their bodies. */
export function serveBuckets(host: BucketHost) {
    return bucket.handle({
        files: (call) => listFiles(host, call),
        file: (call) => headFile(host, call),
        open: { authorize: authorizeOpen, handler: (call) => openLease(host, call) },
        remove: {
            authorize: async (call) => requireWriter(call),
            handler: (call) => removeFiles(host, call),
        },
        createUpload: {
            authorize: async (call) => requireWriter(call),
            handler: (call) => createUpload(host, call),
        },
        uploadPart: {
            authorize: async (call) => requireWriter(call),
            handler: (call) => uploadPart(host, call),
        },
        completeUpload: {
            authorize: async (call) => requireWriter(call),
            handler: (call) => completeUpload(host, call),
        },
        abortUpload: {
            authorize: async (call) => requireWriter(call),
            handler: (call) => abortUpload(host, call),
        },
    });
}

/** List a page of a bucket's files with their metadata. */
function listFiles(
    host: BucketHost,
    call: PreparedCallOf<typeof bucket, "files">,
): Promise<ResultOf<typeof bucket, "files">> {
    return files(host, call, async (opened) => {
        // list a page of files with their metadata
        const input = call.input;
        const page = await opened.list({
            ...(input.prefix === undefined ? {} : { prefix: input.prefix }),
            ...(input.delimiter === undefined ? {} : { delimiter: input.delimiter }),
            ...(input.cursor === undefined ? {} : { cursor: input.cursor }),
            limit: input.limit,
            include: ["httpMetadata", "customMetadata"],
        });

        return {
            files: page.files.map((file) => describe(file)),
            delimitedPrefixes: page.delimitedPrefixes,
            cursor: page.truncated ? page.cursor : null,
        };
    });
}

/** Read one file's metadata, null for a missing file. */
function headFile(
    host: BucketHost,
    call: PreparedCallOf<typeof bucket, "file">,
): Promise<ResultOf<typeof bucket, "file">> {
    return files(host, call, async (opened) => {
        const file = await opened.head(call.input.key);

        return file === null ? null : describe(file);
    });
}

/** Require the right to upload for a write, and the system for a bucket it alone writes. */
async function authorizeOpen(call: CallOf<typeof bucket, "open">): Promise<void> {
    if (call.input.mode === "write") {
        await call.requireAuthorization().require(bucket.permission("write"), call.reference());
        requireWriter(call);
    }
}

/** Presign a lease reading or writing one file's body. */
function openLease(
    host: BucketHost,
    call: PreparedCallOf<typeof bucket, "open">,
): Promise<ResultOf<typeof bucket, "open">> {
    // bind a read's range and entity tag
    const input = call.input;
    const headers = new Headers();
    if (input.mode === "read") {
        if (input.range !== undefined) {
            headers.set("range", input.range);
        }
    }
    // bind a write's body length, stored metadata and replacement precondition
    else {
        if (input.size === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "a write states its body's size",
            });
        }
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
    encrypt(headers, input.customerKey);

    return presign(host, call, { mode: input.mode, key: input.key, headers });
}

/** Delete some files of a bucket. */
function removeFiles(
    host: BucketHost,
    call: PreparedCallOf<typeof bucket, "remove">,
): Promise<ResultOf<typeof bucket, "remove">> {
    return files(host, call, async (opened) => {
        await opened.delete(call.input.keys);

        return {};
    });
}

/** Create a multipart upload with the metadata its file receives. */
function createUpload(
    host: BucketHost,
    call: PreparedCallOf<typeof bucket, "createUpload">,
): Promise<ResultOf<typeof bucket, "createUpload">> {
    return files(host, call, async (opened) => {
        // create the upload with the metadata its file receives
        const input = call.input;
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
    call: PreparedCallOf<typeof bucket, "uploadPart">,
): Promise<ResultOf<typeof bucket, "uploadPart">> {
    // bind the part's length and number
    const input = call.input;
    const headers = new Headers({ "content-length": String(input.size) });
    encrypt(headers, input.customerKey);

    return presign(host, call, {
        mode: "write",
        key: input.key,
        headers,
        query: { partNumber: String(input.partNumber), uploadId: input.uploadId },
    });
}

/** Complete a multipart upload from its parts. */
function completeUpload(
    host: BucketHost,
    call: PreparedCallOf<typeof bucket, "completeUpload">,
): Promise<ResultOf<typeof bucket, "completeUpload">> {
    return files(host, call, async (opened) => {
        // assemble the parts by their entity tags without quotes
        const input = call.input;
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
    call: PreparedCallOf<typeof bucket, "abortUpload">,
): Promise<ResultOf<typeof bucket, "abortUpload">> {
    return files(host, call, async (opened) => {
        const input = call.input;
        await opened.resumeMultipartUpload(input.key, input.uploadId).abort();

        return {};
    });
}

/** A call of a bucket's method. */
type BucketCall = Call<(typeof bucket)["table"]>;

/** Read the bucket a call targets. */
function reference(call: BucketCall): BucketReference {
    return {
        scope: schema.identifier("space").parse(call.scope),
        bucketId: schema.identifier("bucket").parse(call.requireTarget().id),
    };
}

/** Refuse a write into a bucket the system alone writes, unless the system calls. */
function requireWriter(call: BucketCall): void {
    const target = call.requireTarget();
    if (BucketSpec.parse(target.spec).write === "system" && !call.requireAuthorization().isSystem) {
        throw new ServiceError("FORBIDDEN", { message: `the system alone writes ${target.id}` });
    }
}

/** Work on the bucket a call targets, reporting its storage failures as service failures. */
function files<Value>(
    host: BucketHost,
    call: BucketCall,
    work: (opened: Awaited<ReturnType<BucketHost["open"]>>) => Promise<Value>,
): Promise<Value> {
    return reported(async () => work(await host.open(reference(call))));
}

/** Run storage work, reporting its storage failures as service failures. */
async function reported<Value>(work: () => Promise<Value>): Promise<Value> {
    try {
        return await work();
    } catch (error) {
        throw error instanceof BucketError ? error.toServiceError() : error;
    }
}

/** Presign one S3 request on the bucket a call targets. */
async function presign(host: BucketHost, call: BucketCall, lease: LeaseRequest): Promise<Lease> {
    // address the key, and the part the query selects, at the bucket's S3 location
    const { location, credentials } = await reported(() =>
        host.locate(reference(call), lease.mode),
    );
    const url = S3Location.url(location, lease.key);
    for (const [name, value] of Object.entries(lease.query ?? {})) {
        url.searchParams.set(name, value);
    }

    // refuse a URL that left the bucket's path, which only a malformed key causes
    const bucketPath = `${S3Location.url(location).pathname}/`;
    if (!url.pathname.startsWith(bucketPath)) {
        throw new TypeError(`key ${lease.key} leaves its bucket's path`);
    }

    // sign the request, keeping the headers the caller sends
    const now = Date.now();
    const method = LEASE_METHODS[lease.mode];
    const request = new Request(url.href, { method, headers: lease.headers });
    const signature = new SignatureV4({ region: location.region });
    const presigned = await signature.presign(request, credentials, LEASE_LIFETIME, now);

    return {
        url: presigned.url,
        mode: lease.mode,
        headers: Object.fromEntries(presigned.headers),
        expiresAt: now + LEASE_LIFETIME * 1000,
    };
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
    const encoder = new TextEncoder();
    const size = Object.entries(metadata).reduce(
        (total, [name, value]) =>
            total + encoder.encode(name).byteLength + encoder.encode(value).byteLength,
        0,
    );
    if (size > MAX_METADATA_BYTES) {
        throw new ServiceError("BAD_REQUEST", {
            message: `custom metadata is at most ${MAX_METADATA_BYTES} bytes`,
        });
    }
}

/** Bind a customer key as SSE-C headers, when the caller sent one. */
function encrypt(headers: Headers, customerKey: string | undefined): void {
    if (customerKey !== undefined) {
        for (const [name, value] of Object.entries(customerKeyHeaders(customerKey))) {
            headers.set(name, value);
        }
    }
}
