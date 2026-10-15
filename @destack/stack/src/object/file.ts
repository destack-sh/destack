import { MAX_BATCH_FILES, MAX_KEY_BYTES, MAX_PART_NUMBER, STORAGE_CLASSES } from "@destack/bucket";
import { Instant, schema } from "@destack/schema";

/** The largest single upload or part in bytes, S3's 5 GiB limit. */
const MAX_UPLOAD_BYTES = 5 * 1024 ** 3;

/** The longest Range header value, room for two 64-bit offsets. */
const MAX_RANGE_LENGTH = 64;

/** The base64 length of a 256-bit customer key: 32 bytes in 44 characters. */
const CUSTOMER_KEY_LENGTH = 44;

/** A lowercase HTTP header token, the form S3 stores metadata names in (RFC 9110 5.6.2). */
const METADATA_NAME = /^[!#$%&'*+.^_`|~0-9a-z-]+$/u;

/** A customer-provided AES-256 key encrypting a file, as base64. */
export const CustomerKey = schema.sensitive(schema.base64().length(CUSTOMER_KEY_LENGTH));

/** A file key a presigned URL keeps intact: no `.` or `..` segments, which URL paths drop. */
export const FileKey = schema
    .string()
    .min(1)
    .max(MAX_KEY_BYTES)
    .regex(
        /^(?:|[^/.][^/]*|\.[^/.][^/]*|\.\.[^/]+)(?:\/(?:|[^/.][^/]*|\.[^/.][^/]*|\.\.[^/]+))*$/u,
        "file keys refuse . and .. segments",
    );

/** A key prefix or delimiter, up to a key's length. */
const KeyPart = schema.string().max(MAX_KEY_BYTES);

/** A file's own metadata, sent as `x-amz-meta-` headers, at most 2048 bytes in all. */
export const CustomMetadata = schema.record(schema.string().regex(METADATA_NAME), schema.string());

/** A file's key, version, size, entity tag and stored metadata. */
export const FileMetadata = schema.object({
    /** The file key. */
    key: schema.string(),
    /** The version the bucket stored. */
    version: schema.string(),
    /** The body length in bytes. */
    size: schema.number().int().nonnegative(),
    /** The entity tag, unquoted. */
    etag: schema.string(),
    /** The upload time, in UTC milliseconds. */
    uploaded: Instant,
    /** The HTTP headers the file is served with. */
    httpMetadata: schema.object({
        /** The Content-Type header. */
        contentType: schema.string().exactOptional(),
        /** The Content-Language header. */
        contentLanguage: schema.string().exactOptional(),
        /** The Content-Disposition header. */
        contentDisposition: schema.string().exactOptional(),
        /** The Content-Encoding header. */
        contentEncoding: schema.string().exactOptional(),
        /** The Cache-Control header. */
        cacheControl: schema.string().exactOptional(),
        /** The Expires header, in UTC milliseconds. */
        cacheExpiry: Instant.exactOptional(),
    }),
    /** The caller's own metadata, sent as `x-amz-meta-` headers. */
    customMetadata: CustomMetadata,
});
/** A file's key, version, size, entity tag and stored metadata. */
export type FileMetadata = schema.Infer<typeof FileMetadata>;

/** A listed file: its key, size, entity tag, upload time and storage class, as S3 lists it. */
export const FileEntry = FileMetadata.pick({
    key: true,
    size: true,
    etag: true,
    uploaded: true,
}).extend({
    /** The storage class. */
    storageClass: schema.enum(STORAGE_CLASSES),
});

/** A page of a bucket's files and the prefixes its delimiter groups. */
export const FilePage = schema.object({
    /** The files in key order. */
    files: schema.array(FileEntry),
    /** The key prefixes the delimiter grouped. */
    delimitedPrefixes: schema.array(schema.string()),
    /** Where the next page starts, null on the last one. */
    cursor: schema.string().nullable(),
});

/** The file methods' inputs, beside the bucket they target. */
export const FileInput = {
    /** List a page of files. */
    files: schema.object({
        /** The key prefix to list below. */
        prefix: KeyPart.exactOptional(),
        /** The delimiter grouping keys into prefixes. */
        delimiter: KeyPart.exactOptional(),
        /** Where a previous page ended. */
        cursor: schema.string().exactOptional(),
        /** The most files and prefixes in the page. */
        limit: schema.number().int().min(1).max(MAX_BATCH_FILES),
    }),
    /** Read one file's metadata. */
    file: schema.object({
        /** The file key. */
        key: FileKey,
    }),
    /** Open a file for reading or writing through a presigned lease. */
    open: schema.discriminatedUnion("mode", [
        schema.object({
            /** Read the file's body. */
            mode: schema.literal("read"),
            /** The file key. */
            key: FileKey,
            /** The Range header the read binds. */
            range: schema.string().max(MAX_RANGE_LENGTH).exactOptional(),
            /** The entity tag the file must have. */
            ifMatch: schema.string().exactOptional(),
            /** The customer key encrypting the file. */
            customerKey: CustomerKey.exactOptional(),
        }),
        schema.object({
            /** Write the file's body. */
            mode: schema.literal("write"),
            /** The file key. */
            key: FileKey,
            /** The body length in bytes the write sends. */
            size: schema.number().int().nonnegative().max(MAX_UPLOAD_BYTES),
            /** The HTTP headers the written file is served with. */
            httpMetadata: FileMetadata.shape.httpMetadata.exactOptional(),
            /** The caller's own metadata of the written file. */
            customMetadata: CustomMetadata.exactOptional(),
            /** The entity tag the file must have. */
            ifMatch: schema.string().exactOptional(),
            /** Refuse a write replacing an existing file. */
            ifNoneMatch: schema.literal("*").exactOptional(),
            /** The customer key encrypting the file. */
            customerKey: CustomerKey.exactOptional(),
        }),
    ]),
    /** Remove files. */
    remove: schema.object({
        /** The file keys. */
        keys: schema.array(FileKey).min(1).max(MAX_BATCH_FILES),
    }),
    /** Create a multipart upload. */
    createUpload: schema.object({
        /** The file key. */
        key: FileKey,
        /** The HTTP headers the file is served with. */
        httpMetadata: FileMetadata.shape.httpMetadata,
        /** The caller's own metadata. */
        customMetadata: CustomMetadata,
        /** The customer key encrypting the file. */
        customerKey: CustomerKey.exactOptional(),
    }),
    /** Presign one part of a multipart upload. */
    uploadPart: schema.object({
        /** The file key. */
        key: FileKey,
        /** The upload the bucket created. */
        uploadId: schema.string(),
        /** The part's number. */
        partNumber: schema.number().int().min(1).max(MAX_PART_NUMBER),
        /** The part's length in bytes. */
        size: schema.number().int().positive().max(MAX_UPLOAD_BYTES),
        /** The customer key encrypting the file. */
        customerKey: CustomerKey.exactOptional(),
    }),
    /** Complete a multipart upload. */
    completeUpload: schema.object({
        /** The file key. */
        key: FileKey,
        /** The upload the bucket created. */
        uploadId: schema.string(),
        /** The uploaded parts in order. */
        parts: schema
            .array(
                schema.object({
                    /** The part's number. */
                    partNumber: schema.number().int().min(1).max(MAX_PART_NUMBER),
                    /** The part's entity tag. */
                    etag: schema.string(),
                }),
            )
            .min(1)
            .max(MAX_PART_NUMBER),
    }),
    /** Abort a multipart upload. */
    abortUpload: schema.object({
        /** The file key. */
        key: FileKey,
        /** The upload the bucket created. */
        uploadId: schema.string(),
    }),
};
