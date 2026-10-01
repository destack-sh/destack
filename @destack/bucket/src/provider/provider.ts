import { pathToFileURL } from "node:url";
import { type Provider, type Provisioning, type Copying } from "@destack/resource";
import { defineSchema, identifier, schema } from "@destack/schema";
import {
    type BucketFile,
    BucketHttpMetadata,
    MAX_BATCH_FILES,
    STORAGE_CLASSES,
} from "../bucket/index.ts";
import { StorageError } from "../error/index.ts";
import type { LocalBucketHost } from "../local/index.ts";
import { bucket } from "../object/index.ts";
import { S3Location, SignatureV4 } from "../s3/index.ts";
import { BucketKind } from "../declare/bucket.ts";

/**
 * The most bytes one chunk of a bucket copy names: one GiB.
 *
 * A target fetching one GiB at 10 MB/s takes about 110 seconds, well within a presigned URL's lifetime.
 */
const CHUNK_BYTES = 1024 ** 3;

/** How long a presigned file URL a transfer's target fetches stays valid, in seconds: fifteen minutes, eight times a chunk's fetch at 10 MB/s. */
const FILE_URL_SECONDS = 900;

/** How many files a target fetches at once: sixteen keep a 50 ms round trip to about 3 s for a thousand files. */
const FETCH_CONCURRENCY = 16;

/** Where a bucket export continues: the stage it read, and the last key it listed. */
const FileCursor = defineSchema(
    schema.object({
        /** The stage the export read. */
        stage: schema.enum(["live", "fenced"]),
        /** The last key listed, absent at the bucket's start. */
        key: schema.string().optional(),
    }),
);

/** The files one chunk of a bucket copy carries, each fetched from its presigned URL. */
const FileRange = defineSchema(
    schema.object({
        /** The key the range follows, absent from the bucket's start. */
        after: schema.string().optional(),
        /** The last key of the range, absent to the bucket's end. */
        last: schema.string().optional(),
        /** The range's files. */
        files: schema.array(
            schema.object({
                /** The file's key. */
                key: schema.string(),
                /** The entity tag, which the target keeps. */
                etag: schema.string(),
                /** The version, which the target keeps. */
                version: schema.string(),
                /** The upload time in UTC milliseconds, which the target keeps. */
                uploaded: schema.number().int(),
                /** The storage class. */
                storageClass: schema.enum(STORAGE_CLASSES),
                /** The presigned URL reading the file from the source. */
                url: schema.url(),
                /** The HTTP metadata, its expiry as an ISO time. */
                httpMetadata: schema.record(schema.string(), schema.string()),
                /** The application metadata. */
                customMetadata: schema.record(schema.string(), schema.string()),
            }),
        ),
    }),
);
/** The files one chunk of a bucket copy carries. */
type FileRange = schema.Infer<typeof FileRange>;

/** Provide buckets as directories of a host's local buckets, one per resource. */
export function localBucketProvider(
    buckets: LocalBucketHost,
): Provider<typeof BucketKind, typeof bucket> &
    Provisioning<typeof BucketKind> &
    Copying<typeof BucketKind> {
    return {
        kind: BucketKind,
        code: "local",
        facet: bucket,
        provision: async (resource) => {
            // create the bucket's directory
            const opened = { resourceId: identifier("resource").parse(resource.id) };
            await buckets.open(opened);

            return { reference: pathToFileURL(buckets.path(opened)).href };
        },
        export: async function* (copy, after, signal) {
            // list again from the start once fenced, since files have no log of their changes
            const cursor = after === undefined ? undefined : FileCursor.parse(JSON.parse(after));
            let key = cursor?.stage === copy.stage ? cursor.key : undefined;
            const reference = {
                scope: identifier("space").parse(copy.record.scope),
                resourceId: identifier("resource").parse(copy.record.id),
            };
            const source = await buckets.open(reference);
            const endpoint = await buckets.locate(reference);
            const signature = new SignatureV4({ region: endpoint.location.region });

            // name each page's files by presigned URLs, at most a chunk's bytes, until the listing ends
            let isListed = false;
            while (!isListed && !signal.aborted) {
                const page = await source.list({
                    ...(key === undefined ? {} : { startAfter: key }),
                    include: ["httpMetadata", "customMetadata"],
                });
                const listed = chunkOf(page.files);
                const now = Date.now();
                const files = await Promise.all(
                    listed.map(async (file) => ({
                        key: file.key,
                        etag: file.etag,
                        version: file.version,
                        uploaded: file.uploaded.getTime(),
                        storageClass: file.storageClass,
                        url: (
                            await signature.presign(
                                new Request(S3Location.url(endpoint.location, file.key).href),
                                endpoint.credentials,
                                FILE_URL_SECONDS,
                                now,
                            )
                        ).url,
                        httpMetadata: BucketHttpMetadata.encode(file.httpMetadata),
                        customMetadata: file.customMetadata,
                    })),
                );
                isListed = !page.truncated && listed.length === page.files.length;
                const last = isListed ? undefined : listed.at(-1)!.key;
                const range: FileRange = {
                    ...(key === undefined ? {} : { after: key }),
                    ...(last === undefined ? {} : { last }),
                    files,
                };
                key = last;
                yield {
                    cursor: JSON.stringify({
                        stage: copy.stage,
                        ...(last === undefined ? {} : { key: last }),
                    }),
                    body: range,
                };
            }
        },
        import: async (copy, chunk) => {
            // remove the target's other files in the range, a batch at a time
            const range = FileRange.parse(chunk.body);
            const target = await buckets.open({
                resourceId: identifier("resource").parse(copy.record.id),
            });
            const kept = new Set(range.files.map((file) => file.key));
            const present = await target.keys(range);
            const stale = present.filter((key) => !kept.has(key));
            for (let start = 0; start < stale.length; start += MAX_BATCH_FILES) {
                await target.delete(stale.slice(start, start + MAX_BATCH_FILES));
            }

            // fetch each file the target lacks or has another version of, several at once
            let next = 0;
            await Promise.all(
                Array.from(
                    { length: Math.min(FETCH_CONCURRENCY, range.files.length) },
                    async () => {
                        while (next < range.files.length) {
                            const { url, httpMetadata, uploaded, ...file } = range.files[next++]!;
                            await target.fetch(url, {
                                ...file,
                                uploaded: new Date(uploaded),
                                httpMetadata: BucketHttpMetadata.decode(httpMetadata),
                            });
                        }
                    },
                ),
            );
        },
        destroy: (resource) =>
            buckets.destroy({ resourceId: identifier("resource").parse(resource.id) }),
    };
}

/** Take a page's files up to a chunk's bytes, at least one. */
function chunkOf(files: readonly BucketFile[]): BucketFile[] {
    // refuse a file only its customer key reads, which no presigned URL fetches
    const chunk: BucketFile[] = [];
    let bytes = 0;
    for (const file of files) {
        if (file.ssecKeyMd5 !== undefined) {
            throw new StorageError(
                "UNSUPPORTED",
                `transferring ${file.key}, encrypted with a customer key, is not supported`,
            );
        } else if (chunk.length > 0 && bytes + file.size > CHUNK_BYTES) {
            break;
        }
        chunk.push(file);
        bytes += file.size;
    }

    return chunk;
}
