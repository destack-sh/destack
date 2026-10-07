import type { DatabaseConnection, Table } from "@destack/db";
import type { LeaseMode } from "@destack/resource";
import type { Identifier } from "@destack/schema";
import { BucketError } from "../error/index.ts";
import type { BucketEndpoint, BucketHost, BucketReference, S3Credentials } from "../s3/index.ts";
import type { CatalogueBucket } from "./bucket.ts";

/** The databases a host keeps its buckets' catalogues in, one each, such as one schema each in a cell's database. */
export interface CatalogueStore {
    /** Report whether a bucket has a catalogue. */
    has(bucketId: Identifier<"bucket">): Promise<boolean>;
    /** Connect to a bucket's catalogue declaring some tables, creating an empty database for it when missing. */
    connect(
        bucketId: Identifier<"bucket">,
        tables: readonly Table[],
    ): Promise<DatabaseConnection & { close(): Promise<void> }>;
    /** Remove a bucket's catalogue. */
    destroy(bucketId: Identifier<"bucket">): Promise<void>;
}

/** A host keeping buckets over catalogues, each opened once at a time, and serving them over S3 at one endpoint. */
export abstract class CatalogueBucketHost<Opened extends CatalogueBucket = CatalogueBucket>
    implements BucketHost, AsyncDisposable
{
    /** The provider code of the host's buckets, such as `local` or `r2`. */
    abstract readonly provider: string;
    /** The URL serving the host's buckets over S3. */
    readonly endpoint: URL;
    /** The region requests are signed for. */
    readonly region: string;
    /** Read the S3 credentials of a space, absent for a space the host keeps no credentials of. */
    readonly credentials: (space: Identifier<"space">) => Promise<S3Credentials | undefined>;
    /** The opening and opened buckets, by resource. */
    readonly #buckets = new Map<Identifier<"bucket">, Promise<Opened>>();

    /** Serve a host's buckets over S3 at one endpoint. */
    constructor(options: {
        /** The URL serving the host's buckets over S3. */
        readonly endpoint: URL;
        /** The region requests are signed for. */
        readonly region: string;
        /** Read the S3 credentials of a space, absent for a space the host keeps no credentials of. */
        readonly credentials: (space: Identifier<"space">) => Promise<S3Credentials | undefined>;
    }) {
        // keep the endpoint, the region and the credentials
        this.endpoint = options.endpoint;
        this.region = options.region;
        this.credentials = options.credentials;
    }

    /** Name where a bucket lives. */
    abstract reference(bucket: Pick<BucketReference, "bucketId">): string;

    /** Open the bucket an S3 request addresses by its resource, absent when the host has none. */
    abstract named(name: string): Promise<Opened | undefined>;

    /** Fence a bucket while a transfer copies it, across restarts until lifted, returning once its writes in flight finished, and report whether this call set the fence. */
    abstract fence(bucket: Pick<BucketReference, "bucketId">): Promise<boolean>;

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

    /** Locate a bucket at the host's S3 endpoint, with its space's credentials presigning a transfer, refusing a write into a fenced bucket. */
    async locate(bucket: BucketReference, mode: LeaseMode): Promise<BucketEndpoint> {
        // refuse a write into a fenced bucket
        if (mode === "write") {
            (await this.open(bucket)).checkWritable();
        }

        // presign with the credentials of the bucket's space
        const credentials = await this.credentials(bucket.scope);
        if (credentials === undefined) {
            throw new BucketError(
                "NO_SUCH_BUCKET",
                `this host keeps no credentials of ${bucket.scope}`,
            );
        }

        return {
            location: { endpoint: this.endpoint, bucket: bucket.bucketId, region: this.region },
            credentials,
        };
    }

    /** Count the bytes a bucket's published files keep, opening it when closed. */
    async bytes(bucket: Pick<BucketReference, "bucketId">): Promise<number> {
        return (await this.open(bucket)).bytes();
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
