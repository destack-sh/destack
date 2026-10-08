import type {
    Bucket,
    BucketBody,
    BucketCondition,
    BucketFile,
    BucketHttpMetadata,
    BucketRange,
    MultipartOptions,
    MultipartUpload,
    StorageClass,
    UploadedPart,
    UploadPartOptions,
} from "../bucket/index.ts";

/** A bucket that backs the S3 protocol directly, with every S3 operation in scope. */
export interface S3Bucket extends Bucket {
    /** Copy a file, optionally under preconditions on the source, refusing a failed one with `PRECONDITION_FAILED`. */
    copy(source: string, destination: string, options?: BucketCopyOptions): Promise<BucketFile>;
    /** List incomplete multipart uploads in key order. */
    listUploads(options?: UploadListOptions): Promise<UploadListing>;
    /** Create an incomplete file upload with the operations S3 adds. */
    createMultipartUpload(key: string, options?: MultipartOptions): Promise<S3MultipartUpload>;
    /** Reference an existing upload with the operations S3 adds. */
    resumeMultipartUpload(key: string, uploadId: string): S3MultipartUpload;
}

/** An incomplete file upload with the operations S3 adds. */
export interface S3MultipartUpload extends MultipartUpload {
    /** Store or replace one numbered part, describing it as S3 lists it. */
    uploadPart(partNumber: number, body: BucketBody, options?: UploadPartOptions): Promise<Part>;
    /** Store a numbered part copied from a file, refusing a failed source precondition with `PRECONDITION_FAILED`. */
    uploadPartCopy(partNumber: number, source: string, options?: PartCopyOptions): Promise<Part>;
    /** List a page of the stored parts. */
    listParts(options?: PartListOptions): Promise<PartListing>;
}

/** Copy options. */
export interface BucketCopyOptions {
    /** Preconditions on the source file. */
    onlyIf?: BucketCondition;
    /** Replacement HTTP headers; the source's headers by default. */
    httpMetadata?: BucketHttpMetadata;
    /** Replacement application metadata; the source's metadata by default. */
    customMetadata?: Record<string, string>;
    /** The storage class of the copy, Standard by default. */
    storageClass?: StorageClass;
}

/** Part copy options. */
export interface PartCopyOptions {
    /** The source bytes to copy. */
    range?: BucketRange;
    /** Preconditions on the source file. */
    onlyIf?: BucketCondition;
}

/** A stored part of an incomplete upload. */
export interface Part extends UploadedPart {
    /** The part length in bytes. */
    size: number;
    /** The upload time. */
    uploaded: Date;
}

/** An incomplete multipart upload. */
export interface Upload {
    /** The destination key. */
    key: string;
    /** The upload identifier. */
    uploadId: string;
    /** The creation time. */
    initiated: Date;
    /** The storage class of the completed file. */
    storageClass: StorageClass;
}

/** Upload listing options. */
export interface UploadListOptions {
    /** The required key prefix. */
    prefix?: string;
    /** List only uploads to keys after this key, or to this key after the upload marker. */
    keyMarker?: string;
    /** List only uploads after this upload of the key marker. */
    uploadIdMarker?: string;
    /** The maximum number of uploads, from 1 through 1000. */
    limit?: number;
}

/** Part listing options. */
export interface PartListOptions {
    /** List only parts numbered above this. */
    partNumberMarker?: number;
    /** The maximum number of parts, from 1 through 1000. */
    limit?: number;
}

/** A page of incomplete uploads ordered by key, then upload identifier. */
export interface UploadListing {
    /** The listed uploads. */
    uploads: Upload[];
    /** Whether more uploads remain after the last listed one. */
    truncated: boolean;
}

/** A page of an upload's parts ordered by part number. */
export interface PartListing {
    /** The listed parts. */
    parts: Part[];
    /** Whether more parts remain after the last listed one. */
    truncated: boolean;
}
