import type { Table } from "@destack/db";
import {
    connectDurableObject,
    DurableObjectDatabaseHost,
    type DurableObjectStorage,
} from "@destack/db/cloudflare";
import { schema, type Identifier } from "@destack/schema";
import type { Bucket } from "../bucket/index.ts";
import {
    CatalogueBucket,
    CatalogueBucketHost,
    type CatalogueStore,
    catalogueDatabase,
} from "../catalogue/index.ts";
import { BucketError } from "../error/index.ts";
import type { BucketReference, S3Credentials } from "../s3/index.ts";
import { R2BlobStore } from "./store.ts";

/** The file below a bucket's prefix marking it fenced while a transfer copies it. */
const FENCE_FILE = "fence";

/** Where a cell keeps its buckets: their blobs in the residency's R2 bucket, their catalogues in its own database, served over S3 at one endpoint. */
export interface R2BucketHostOptions {
    /** The residency's R2 bucket, keeping each bucket's blobs below the bucket's identifier. */
    readonly files: Bucket;
    /** The name of the residency's R2 bucket, such as `destack-production-files-eu`. */
    readonly name: string;
    /** The databases of the buckets' catalogues. */
    readonly catalogues: CatalogueStore;
    /** The URL serving the host's buckets over S3. */
    readonly endpoint: URL;
    /** The region requests are signed for. */
    readonly region: string;
    /** Read the S3 credentials of a space, absent for a space the host keeps no credentials of. */
    readonly credentials: (space: Identifier<"space">) => Promise<S3Credentials | undefined>;
}

/** A cell keeping buckets: each one a prefix of the residency's R2 bucket with its catalogue in the cell's database, opened once, and served over S3 at one endpoint. */
export class R2BucketHost extends CatalogueBucketHost {
    /** The provider code of R2 buckets. */
    readonly provider: string;
    /** The residency's R2 bucket. */
    readonly #files: Bucket;
    /** The name of the residency's R2 bucket. */
    readonly name: string;
    /** The databases of the buckets' catalogues. */
    readonly #catalogues: CatalogueStore;

    /** Keep a cell's buckets in the residency's R2 bucket and the cell's database. */
    constructor(options: R2BucketHostOptions) {
        // keep the residency's bucket and the catalogues
        super(options);
        this.provider = "r2";
        this.#files = options.files;
        this.name = options.name;
        this.#catalogues = options.catalogues;
    }

    /** Name where a bucket's blobs live: its prefix in the residency's R2 bucket. */
    override reference(bucket: Pick<BucketReference, "bucketId">): string {
        return `r2://${this.name}/${bucket.bucketId}/`;
    }

    /** Open the bucket an S3 request addresses by its resource, absent when the host has none. */
    override async named(name: string): Promise<CatalogueBucket | undefined> {
        const bucketId = schema.identifier("bucket").safeParse(name);
        if (!bucketId.success || !(await this.#catalogues.has(bucketId.data))) {
            return undefined;
        }

        return this.open({ bucketId: bucketId.data });
    }

    /** Fence a bucket while a transfer copies it, across restarts until lifted, returning once its writes in flight finished, and report whether this call set the fence. */
    override async fence(bucket: Pick<BucketReference, "bucketId">): Promise<boolean> {
        // mark the bucket fenced below its prefix, then refuse its writes
        const opened = await this.open(bucket);
        await this.#files.put(`${bucket.bucketId}/${FENCE_FILE}`, new Uint8Array());

        return await opened.fence();
    }

    /** Lift a bucket's fence, accepting its writes again. */
    override async lift(bucket: Pick<BucketReference, "bucketId">): Promise<void> {
        // remove the fence file, then accept the bucket's writes
        const opened = await this.open(bucket);
        await this.#files.delete(`${bucket.bucketId}/${FENCE_FILE}`);
        await opened.lift();
    }

    /** Close a bucket, then delete its blobs and its catalogue. */
    override async destroy(bucket: Pick<BucketReference, "bucketId">): Promise<void> {
        await this.close(bucket);
        await new R2BlobStore(this.#files, `${bucket.bucketId}/`).clear();
        await this.#catalogues.destroy(bucket.bucketId);
    }

    /** Open a bucket's catalogue, created in its scope once, over its blobs below its prefix. */
    protected override async load(
        { bucketId }: Pick<BucketReference, "bucketId">,
        scope: string | undefined,
    ): Promise<CatalogueBucket> {
        // refuse creating a bucket without the scope it belongs to
        const isNew = !(await this.#catalogues.has(bucketId));
        if (isNew && scope === undefined) {
            throw new BucketError("NO_SUCH_BUCKET", `no bucket ${bucketId}`);
        }

        // read whether a transfer fenced the bucket
        const isFenced = (await this.#files.head(`${bucketId}/${FENCE_FILE}`)) !== null;

        // connect to the catalogue, logging it in its scope once, and migrate it
        const tables = catalogueDatabase.tables;
        let database = await this.#catalogues.connect(bucketId, tables);
        try {
            if (isNew) {
                await database.log.create(scope);
            }
            await database.migrate(tables);
        } catch (error) {
            await database.close();
            throw error;
        }

        // keep the bucket, reconnecting its catalogue over the tables beside its own
        const opened = CatalogueBucket.create({
            get database() {
                return database;
            },
            blobs: new R2BlobStore(this.#files, `${bucketId}/`),
            migrate: async (beside) => {
                // migrate, then reconnect declaring every table
                const migrated = [...tables, ...beside];
                await database.migrate(migrated);
                await database.close();
                database = await this.#catalogues.connect(bucketId, migrated);
            },
            close: () => database.close(),
        });

        // refuse writes while the fence file remains from a transfer
        if (isFenced) {
            await opened.fence();
        }

        return opened;
    }
}

/** The catalogues of a cell's buckets in its Durable Object's storage, each in its bucket's namespace. */
export class DurableObjectCatalogueStore implements CatalogueStore {
    /** The object's databases. */
    readonly #databases: DurableObjectDatabaseHost;

    /** Keep catalogues in an object's storage. */
    constructor(storage: DurableObjectStorage) {
        this.#databases = new DurableObjectDatabaseHost(storage);
    }

    /** Report whether a bucket has a catalogue. */
    async has(bucketId: Identifier<"bucket">): Promise<boolean> {
        return (await this.#databases.relations(bucketId)).length > 0;
    }

    /** Connect to a bucket's catalogue declaring some tables in its namespace. */
    async connect(bucketId: Identifier<"bucket">, tables: readonly Table[]) {
        return connectDurableObject(this.#databases.storage, tables, { namespace: bucketId });
    }

    /** Remove a bucket's catalogue. */
    async destroy(bucketId: Identifier<"bucket">): Promise<void> {
        await this.#databases.drop(bucketId);
    }
}
