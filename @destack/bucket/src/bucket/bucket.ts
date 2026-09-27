import type { BucketFile, BucketFileBody, StorageClass } from "./file.ts";
import type { BucketHttpMetadata } from "./metadata.ts";
import type { MultipartOptions, MultipartUpload } from "./multipart.ts";
import type { BucketCondition } from "./condition.ts";
import type { BucketRange } from "./range.ts";
import type { BucketListing } from "./list.ts";

/** File storage supplied by the host. */
export interface Bucket {
    /** Read metadata; return null when the key does not exist. */
    head(key: string): Promise<BucketFile | null>;
    /** Read a file without conditional metadata responses. */
    get(
        key: string,
        options?: BucketGetOptions & { onlyIf?: undefined },
    ): Promise<BucketFileBody | null>;
    /** Read a file; return metadata alone when a precondition fails. */
    get(key: string, options: BucketGetOptions): Promise<BucketFileBody | BucketFile | null>;
    /** Store a file without preconditions. */
    put(
        key: string,
        body: BucketBody,
        options?: BucketPutOptions & { onlyIf?: undefined },
    ): Promise<BucketFile>;
    /** Store a complete file and atomically replace any previous value. */
    put(key: string, body: BucketBody, options: BucketPutOptions): Promise<BucketFile | null>;
    /** Delete up to 1000 files; absent keys remain absent. */
    delete(key: string | string[]): Promise<void>;
    /** List a page of metadata. */
    list(options?: BucketListOptions): Promise<BucketListing>;
    /** Create an incomplete file upload. */
    createMultipartUpload(key: string, options?: MultipartOptions): Promise<MultipartUpload>;
    /** Reference an existing upload; subsequent operations check its existence. */
    resumeMultipartUpload(key: string, uploadId: string): MultipartUpload;
}

/** Read options. */
export interface BucketGetOptions {
    /** The preconditions the current file must meet to return its body. */
    onlyIf?: BucketCondition;
    /** The bytes to return. */
    range?: BucketRange;
    /** The customer key encrypting the file, as 32 raw bytes or 64 hexadecimal digits. */
    ssecKey?: ArrayBuffer | string;
}

/** Write options. */
export interface BucketPutOptions {
    /** The preconditions the current file, or its absence, must meet to be replaced. */
    onlyIf?: BucketCondition;
    /** Stored HTTP headers. */
    httpMetadata?: BucketHttpMetadata;
    /** Application metadata. */
    customMetadata?: Record<string, string>;
    /** The expected MD5 digest; hexadecimal text or raw bytes. */
    md5?: string | ArrayBuffer | ArrayBufferView;
    /** The expected SHA-1 digest; hexadecimal text or raw bytes. */
    sha1?: string | ArrayBuffer | ArrayBufferView;
    /** The expected SHA-256 digest; hexadecimal text or raw bytes. */
    sha256?: string | ArrayBuffer | ArrayBufferView;
    /** The expected SHA-384 digest; hexadecimal text or raw bytes. */
    sha384?: string | ArrayBuffer | ArrayBufferView;
    /** The expected SHA-512 digest; hexadecimal text or raw bytes. */
    sha512?: string | ArrayBuffer | ArrayBufferView;
    /** The storage class, Standard by default. */
    storageClass?: StorageClass;
    /** The customer key encrypting the file, as 32 raw bytes or 64 hexadecimal digits. */
    ssecKey?: ArrayBuffer | string;
}

/** Listing options. */
export interface BucketListOptions {
    /** The required key prefix. */
    prefix?: string;
    /** The opaque continuation from a previous page with the same prefix. */
    cursor?: string;
    /** List only keys after this key; a cursor takes precedence. */
    startAfter?: string;
    /** The maximum number of files, from 1 through 1000. */
    limit?: number;
    /** Group keys at this delimiter after the prefix. */
    delimiter?: string;
    /** Include the selected metadata in listed files. */
    include?: ("httpMetadata" | "customMetadata")[];
}

/** Bytes accepted by file storage. */
export type BucketBody =
    | ReadableStream<Uint8Array>
    | ArrayBufferView<ArrayBuffer>
    | ArrayBuffer
    | string
    | Blob
    | null;
