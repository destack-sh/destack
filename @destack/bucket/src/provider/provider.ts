import type { DatabaseHandle } from "@destack/db/blob";
import {
    type Fence,
    type Opener,
    type Provider,
    type Provisioner,
    type ResourceRecord,
} from "@destack/resource";
import type { Controller } from "@destack/service/control";
import { schema } from "@destack/schema";
import type { CatalogueBucketHost } from "../catalogue/index.ts";
import type { LocalBucketHost } from "../local/index.ts";
import type { R2BucketHost } from "../r2/index.ts";
import type { BucketReference } from "../s3/index.ts";
import { serveBucket } from "../server/index.ts";
import { BucketKind } from "../declare/bucket.ts";
import { SweepController } from "./sweep.ts";

/** A bucket provider: provisioning, opening and fencing one host's buckets, serving their files and sweeping them. */
type BucketProvider = Provider<
    typeof BucketKind,
    ReturnType<typeof serveBucket>,
    DatabaseHandle,
    never,
    never,
    Controller
> & {
    readonly provision: Provisioner<typeof BucketKind>;
    readonly open: Opener<typeof BucketKind, DatabaseHandle>;
    readonly fence: Fence<typeof BucketKind>;
    readonly controllers: readonly Controller[];
};

/** Bucket providers by backend. */
export const bucketProvider = {
    /** Provide buckets as directories of a device host's local buckets. */
    local: (host: LocalBucketHost): BucketProvider => provide("local", host),
    /** Provide buckets as prefixes of a cell's residency R2 bucket, with their catalogues in the cell's database. */
    r2: (host: R2BucketHost): BucketProvider => provide("r2", host),
};

/** Provide a host's buckets under a provider code, one per resource, swept by the host. */
function provide(code: string, buckets: CatalogueBucketHost): BucketProvider {
    return {
        kind: BucketKind,
        code,
        object: serveBucket(buckets),
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
        controllers: [new SweepController(buckets)],
    };
}

/** Name the bucket a bucket resource is. */
function bucketOf(resource: Pick<ResourceRecord, "id">): Pick<BucketReference, "bucketId"> {
    return { bucketId: schema.identifier("bucket").parse(resource.id) };
}
