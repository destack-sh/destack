import type { BucketBody } from "./bucket.ts";
import type { BucketFile, StorageClass } from "./file.ts";
import type { BucketHttpMetadata } from "./metadata.ts";
import { BucketError } from "../error/index.ts";

/** The highest part number of a multipart upload, the S3 and R2 limit. */
export const MAX_PART_NUMBER = 10000;

/** Metadata applied when a multipart upload completes. */
export interface MultipartOptions {
    /** Stored HTTP headers. */
    httpMetadata?: BucketHttpMetadata;
    /** Application metadata. */
    customMetadata?: Record<string, string>;
    /** The storage class of the completed file, Standard by default. */
    storageClass?: StorageClass;
    /** The customer key encrypting every part, as 32 raw bytes or 64 hexadecimal digits. */
    ssecKey?: ArrayBuffer | string;
}

/** Options of one part upload. */
export interface UploadPartOptions {
    /** The customer key the upload was created with. */
    ssecKey?: ArrayBuffer | string;
}

/** A successfully uploaded numbered part. */
export interface UploadedPart {
    /** The part number, from 1 through 10000. */
    partNumber: number;
    /** The entity tag returned by uploadPart. */
    etag: string;
}

/** A successfully uploaded numbered part. */
export const UploadedPart = { checkNumber };

/** An incomplete file upload retained by its bucket. */
export interface MultipartUpload {
    /** The destination key. */
    readonly key: string;
    /** The upload identifier. */
    readonly uploadId: string;
    /** Store or replace one numbered part. */
    uploadPart(
        partNumber: number,
        body: BucketBody,
        options?: UploadPartOptions,
    ): Promise<UploadedPart>;
    /** Assemble the selected part uploads into one file. */
    complete(uploaded: UploadedPart[]): Promise<BucketFile>;
    /** Discard an incomplete upload. */
    abort(): Promise<void>;
}

/** Require a part number from 1 through 10000. */
function checkNumber(partNumber: number): void {
    if (!Number.isInteger(partNumber) || partNumber < 1 || partNumber > MAX_PART_NUMBER) {
        throw new BucketError(
            "INVALID_PART",
            `part numbers must be integers from 1 through ${MAX_PART_NUMBER}`,
        );
    }
}
