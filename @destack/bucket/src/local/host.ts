import { rm, stat } from "node:fs/promises";
import { join } from "node:path";
import type { Keychain } from "@destack/host/keychain";
import { schema } from "@destack/schema";
import type { BucketEndpoint, BucketHost, BucketReference } from "../s3/index.ts";
import { S3Credentials } from "../s3/index.ts";
import { LocalBucket } from "./bucket.ts";

/** The random bytes of an access key identifier, twenty hexadecimal digits as AWS's are twenty characters. */
const ACCESS_KEY_BYTES = 10;

/** The random bytes of a secret access key, forty base64 characters as AWS's are. */
const SECRET_KEY_BYTES = 30;

/** Where a host serves its buckets over S3, and the credentials presigning their transfers. */
export interface LocalBucketHostOptions {
    /** The directory with one directory per bucket. */
    readonly directory: string;
    /** The URL serving the host's buckets over S3. */
    readonly endpoint: URL;
    /** The region requests are signed for. */
    readonly region: string;
    /** The host's S3 credentials, which never leave it. */
    readonly credentials: S3Credentials;
}

/** A device host keeping buckets: one directory each, opened once, and served over S3 at one endpoint. */
export class LocalBucketHost implements BucketHost, AsyncDisposable {
    /** The directory with one directory per bucket. */
    readonly directory: string;
    /** The URL serving the host's buckets over S3. */
    readonly endpoint: URL;
    /** The region requests are signed for. */
    readonly region: string;
    /** The host's S3 credentials. */
    readonly credentials: S3Credentials;
    /** The opened buckets, by resource, since a bucket's directory opens once at a time. */
    readonly #opened = new Map<string, Promise<LocalBucket>>();

    /** Keep a host's buckets under a directory. */
    constructor(options: LocalBucketHostOptions) {
        // keep the directory, the endpoint and the credentials
        this.directory = options.directory;
        this.endpoint = options.endpoint;
        this.region = options.region;
        this.credentials = options.credentials;
    }

    /** Read the S3 credentials a keychain keeps under a name, generating them once. */
    static async credentials(keychain: Keychain, name: string): Promise<S3Credentials> {
        // read the kept credentials
        const kept = await keychain.load(name);
        if (kept !== undefined) {
            return S3Credentials.parse(JSON.parse(kept));
        }

        // generate and keep new ones
        const credentials = {
            accessKeyId: crypto
                .getRandomValues(new Uint8Array(ACCESS_KEY_BYTES))
                .toHex()
                .toUpperCase(),
            secretAccessKey: crypto.getRandomValues(new Uint8Array(SECRET_KEY_BYTES)).toBase64(),
        };
        await keychain.save(name, JSON.stringify(credentials));

        return credentials;
    }

    /** Build the directory of a bucket. */
    path(bucket: Pick<BucketReference, "bucketId">): string {
        return join(this.directory, bucket.bucketId);
    }

    /** Open a bucket, or create it in the scope it belongs to, reusing the one opened before. */
    open(bucket: Pick<BucketReference, "bucketId">, scope?: string): Promise<LocalBucket> {
        let opened = this.#opened.get(bucket.bucketId);
        if (opened === undefined) {
            opened = LocalBucket.open(this.path(bucket), scope);
            this.#opened.set(bucket.bucketId, opened);
        }

        return opened;
    }

    /** Open the bucket an S3 request addresses by its resource, absent when the host has none. */
    async named(name: string): Promise<LocalBucket | undefined> {
        // refuse a name no bucket takes, and a bucket without a directory
        const bucketId = schema.identifier("bucket").safeParse(name);
        if (!bucketId.success) {
            return undefined;
        }
        const isPresent = await stat(this.path({ bucketId: bucketId.data })).then(
            (found) => found.isDirectory(),
            (error: NodeJS.ErrnoException) => {
                if (error.code === "ENOENT") {
                    return false;
                }
                throw error;
            },
        );

        return isPresent ? this.open({ bucketId: bucketId.data }) : undefined;
    }

    /** Locate a bucket at the host's S3 endpoint, with the credentials presigning its transfers. */
    async locate(bucket: BucketReference): Promise<BucketEndpoint> {
        return {
            location: { endpoint: this.endpoint, bucket: bucket.bucketId, region: this.region },
            credentials: this.credentials,
        };
    }

    /** Close a bucket and remove its directory with its files. */
    async destroy(bucket: Pick<BucketReference, "bucketId">): Promise<void> {
        await this.close(bucket);
        await rm(this.path(bucket), { recursive: true, force: true });
    }

    /** Close a bucket opened before. */
    async close(bucket: Pick<BucketReference, "bucketId">): Promise<void> {
        const opened = this.#opened.get(bucket.bucketId);
        this.#opened.delete(bucket.bucketId);
        await (await opened)?.[Symbol.asyncDispose]();
    }

    /** Close every opened bucket. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const bucketId of this.#opened.keys()) {
            await this.close({ bucketId: schema.identifier("bucket").parse(bucketId) });
        }
    }
}
