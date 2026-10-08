import {
    type Fence,
    type Opener,
    type Provider,
    type Provisioner,
    type ResourceRecord,
    type Snapshotter,
} from "@destack/resource";
import type { Controller } from "@destack/service/control";
import { schema } from "@destack/schema";
import {
    BucketSnapshot,
    type CatalogueBucketHost,
    type CatalogueHandle,
} from "../catalogue/index.ts";
import type { BucketReference } from "../s3/index.ts";
import { BucketKind } from "../declare/index.ts";
import { SweepController } from "./sweep.ts";

/** A bucket provider: provisioning, opening, fencing and snapshotting one host's buckets, serving their files through an object's handlers and sweeping them. */
export type BucketProvider<Object> = Provider<
    typeof BucketKind,
    Object,
    CatalogueHandle,
    never,
    never,
    Controller
> & {
    readonly provision: Provisioner<typeof BucketKind>;
    readonly open: Opener<typeof BucketKind, CatalogueHandle>;
    readonly fence: Fence<typeof BucketKind>;
    readonly snapshot: Snapshotter<typeof BucketKind>;
    readonly controllers: readonly Controller[];
};

/** Provide a host's buckets under its provider code, one per resource, served through an object's handlers and swept by the host. */
export function bucketProvider<Object>(
    buckets: CatalogueBucketHost,
    object: Object,
): BucketProvider<Object> {
    return {
        kind: BucketKind,
        code: buckets.provider,
        object,
        provision: {
            provision: async (resource) => {
                // create the bucket in its space
                await buckets.open(bucketOf(resource), resource.scope);

                return { reference: buckets.reference(bucketOf(resource)) };
            },
            destroy: (resource) => buckets.destroy(bucketOf(resource)),
        },
        open: {
            // open the bucket's catalogue and blobs
            open: async (resource) => (await buckets.open(bucketOf(resource))).database(),
        },
        fence: {
            fence: (resource) => buckets.fence(bucketOf(resource)),
            lift: (resource) => buckets.lift(bucketOf(resource)),
        },
        snapshot: {
            // copy the bucket's catalogue and the blobs of the files it lists
            snapshot: async (resource, _desired, store) =>
                BucketSnapshot.take((await buckets.open(bucketOf(resource))).database(), store),
            restore: async (resource, _desired, digest, store, _recipient, base) => {
                const bucket = (await buckets.open(bucketOf(resource))).database();
                await BucketSnapshot.restore(bucket, digest, store, base);
            },
        },
        controllers: [new SweepController(buckets)],
    };
}

/** Name the bucket a bucket resource is. */
function bucketOf(resource: Pick<ResourceRecord, "id">): Pick<BucketReference, "bucketId"> {
    return { bucketId: schema.identifier("bucket").parse(resource.id) };
}
