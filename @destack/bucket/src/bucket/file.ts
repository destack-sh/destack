import { BucketHttpMetadata } from "./metadata.ts";
import { ContentDigest } from "./digest.ts";
import { schema, type JsonValue } from "@destack/schema";
import { BucketError } from "../error/error.ts";

/** The storage classes R2 offers, in its own spelling. */
export const STORAGE_CLASSES = ["Standard", "InfrequentAccess"] as const;

/** A storage class, trading storage cost against retrieval cost. */
export type StorageClass = (typeof STORAGE_CLASSES)[number];

/** A storage class, trading storage cost against retrieval cost. */
export const StorageClass = { read };

/** Metadata for a specific upload of a file. */
export class BucketFile {
    /** The exact file key. */
    readonly key: string;
    /** The unique upload identifier. */
    readonly version: string;
    /** The complete file length in bytes. */
    readonly size: number;
    /** The unquoted entity tag. */
    readonly etag: string;
    /** The upload time. */
    readonly uploaded: Date;
    /** The stored HTTP headers. */
    readonly httpMetadata: BucketHttpMetadata;
    /** Application metadata. */
    readonly customMetadata: Record<string, string>;
    /** The stored content digest. */
    readonly digest: ContentDigest;
    /** The storage class. */
    readonly storageClass: StorageClass;
    /** The MD5 digest of the customer key encrypting the file, as base64. */
    readonly ssecKeyMd5: string | undefined;
    /** The time before which the file can be neither replaced nor deleted, absent for an unlocked file. */
    readonly retainUntil: Date | undefined;

    /** Retain the uploaded file's metadata. */
    constructor(
        key: string,
        version: string,
        size: number,
        etag: string,
        uploaded: Date,
        httpMetadata: BucketHttpMetadata = {},
        customMetadata: Record<string, string> = {},
        digest: ContentDigest = new ContentDigest(),
        storageClass: StorageClass = "Standard",
        ssecKeyMd5?: string,
        retainUntil?: Date,
    ) {
        // retain the file description
        this.key = key;
        this.version = version;
        this.size = size;
        this.etag = etag;
        this.uploaded = uploaded;
        this.httpMetadata = httpMetadata;
        this.customMetadata = customMetadata;
        this.digest = digest;
        this.storageClass = storageClass;
        this.ssecKeyMd5 = ssecKeyMd5;
        this.retainUntil = retainUntil;
    }

    /** Report whether the file is locked at a time, before its retain-until date. */
    isLocked(now: number): boolean {
        return this.retainUntil !== undefined && this.retainUntil.getTime() > now;
    }

    /** The quoted entity tag for HTTP responses. */
    get httpEtag(): string {
        return `"${this.etag}"`;
    }

    /** Apply the stored HTTP metadata to response headers. */
    writeHttpMetadata(headers: Headers): void {
        BucketHttpMetadata.write(this.httpMetadata, headers);
    }
}

/** A file and its selected byte stream. */
export class BucketFileBody extends BucketFile {
    /** The selected bytes; consume or cancel this stream. */
    readonly body: ReadableStream<Uint8Array>;
    /** The returned range, when requested. */
    readonly range: { offset: number; length: number } | undefined;

    /** The response used for Web body consumption. */
    readonly #response: Response;

    /** Combine metadata and a readable body. */
    constructor(
        file: BucketFile,
        body: ReadableStream<Uint8Array>,
        range?: { offset: number; length: number },
    ) {
        // copy the file description and serve the body through a response
        super(
            file.key,
            file.version,
            file.size,
            file.etag,
            file.uploaded,
            file.httpMetadata,
            file.customMetadata,
            file.digest,
            file.storageClass,
            file.ssecKeyMd5,
        );
        this.body = body;
        this.range = range;
        this.#response = new Response(body);
        this.writeHttpMetadata(this.#response.headers);
    }

    /** Whether the body has been consumed. */
    get bodyUsed(): boolean {
        return this.#response.bodyUsed;
    }

    /** Read the body as an array buffer. */
    arrayBuffer(): Promise<ArrayBuffer> {
        return this.#response.arrayBuffer();
    }

    /** Read the body as bytes. */
    async bytes(): Promise<Uint8Array<ArrayBuffer>> {
        return new Uint8Array(await this.arrayBuffer());
    }

    /** Read the body as UTF-8 text. */
    text(): Promise<string> {
        return this.#response.text();
    }

    /** Read the body as a JSON value. */
    async json(): Promise<JsonValue> {
        return schema.json().parse(await this.#response.json());
    }

    /** Read the body as a blob with its content type. */
    blob(): Promise<Blob> {
        return this.#response.blob();
    }
}

/** Read a storage class name, rejecting classes R2 does not offer. */
function read(value: string): StorageClass {
    const storageClass = STORAGE_CLASSES.find((entry) => entry === value);
    if (storageClass === undefined) {
        throw new BucketError("INVALID_STORAGE_CLASS", `unknown storage class ${value}`);
    }

    return storageClass;
}
