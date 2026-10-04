import type { LeaseMode } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import type { BucketEndpoint, BucketHost, BucketReference, S3Credentials } from "../s3/index.ts";
import type { CatalogueBucket } from "./bucket.ts";

/** A host keeping buckets over catalogues, each opened once at a time, and serving them over S3 at one endpoint. */
export abstract class CatalogueBucketHost<Opened extends CatalogueBucket = CatalogueBucket>
    implements BucketHost, AsyncDisposable
{
    /** The URL serving the host's buckets over S3. */
    readonly endpoint: URL;
    /** The region requests are signed for. */
    readonly region: string;
    /** The host's S3 credentials. */
    readonly credentials: S3Credentials;
    /** The opening and opened buckets, by resource. */
    readonly #buckets = new Map<Identifier<"bucket">, Promise<Opened>>();

    /** Serve a host's buckets over S3 at one endpoint. */
    constructor(options: {
        /** The URL serving the host's buckets over S3. */
        readonly endpoint: URL;
        /** The region requests are signed for. */
        readonly region: string;
        /** The host's S3 credentials, which never leave it. */
        readonly credentials: S3Credentials;
    }) {
        // keep the endpoint, the region and the credentials
        this.endpoint = options.endpoint;
        this.region = options.region;
        this.credentials = options.credentials;
    }

    /** Name where a bucket lives. */
    abstract reference(bucket: Pick<BucketReference, "bucketId">): string;

    /** Fence a bucket while a transfer copies it, across restarts until lifted, returning once its writes in flight finished. */
    abstract fence(bucket: Pick<BucketReference, "bucketId">): Promise<void>;

    /** Lift a bucket's fence, accepting its writes again. */
    abstract lift(bucket: Pick<BucketReference, "bucketId">): Promise<void>;

    /** Close a bucket and delete its files. */
    abstract destroy(bucket: Pick<BucketReference, "bucketId">): Promise<void>;

    /** Open a bucket from its storage, or create it in the scope it belongs to. */
    protected abstract load(
        bucket: Pick<BucketReference, "bucketId">,
        scope: string | undefined,
    ): Promise<Opened>;

    /** Open a bucket, or create it in the scope it belongs to, reusing the one opened before. */
    open(bucket: Pick<BucketReference, "bucketId">, scope?: string): Promise<Opened> {
        // reuse a bucket opened before
        const opened = this.#buckets.get(bucket.bucketId);
        if (opened !== undefined) {
            return opened;
        }

        // open the bucket and keep the opening
        const opening = this.load(bucket, scope);
        this.#buckets.set(bucket.bucketId, opening);

        // forget a failed open, so the next call opens the bucket again
        opening.catch(() => {
            if (this.#buckets.get(bucket.bucketId) === opening) {
                this.#buckets.delete(bucket.bucketId);
            }
        });

        return opening;
    }

    /** Locate a bucket at the host's S3 endpoint, with the credentials presigning a transfer, refusing a write into a fenced bucket. */
    async locate(bucket: BucketReference, mode: LeaseMode): Promise<BucketEndpoint> {
        if (mode === "write") {
            (await this.open(bucket)).checkWritable();
        }

        return {
            location: { endpoint: this.endpoint, bucket: bucket.bucketId, region: this.region },
            credentials: this.credentials,
        };
    }

    /** Close a bucket opened before. */
    async close(bucket: Pick<BucketReference, "bucketId">): Promise<void> {
        const opened = this.#buckets.get(bucket.bucketId);
        this.#buckets.delete(bucket.bucketId);
        await (await opened)?.[Symbol.asyncDispose]();
    }

    /** Sweep each open bucket, answering false when writers kept a sweep from finishing. */
    async sweep(): Promise<boolean> {
        let isSwept = true;
        for (const opened of this.#buckets.values()) {
            // sweep each bucket that opened, leaving a failed opening to its opener
            const [settled] = await Promise.allSettled([opened]);
            if (settled.status === "fulfilled") {
                isSwept = (await settled.value.sweep()) && isSwept;
            }
        }

        return isSwept;
    }

    /** Close every opened bucket. */
    async [Symbol.asyncDispose](): Promise<void> {
        for (const bucketId of this.#buckets.keys()) {
            await this.close({ bucketId });
        }
    }
}
