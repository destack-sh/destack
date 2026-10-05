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
import type { BucketReference } from "../s3/index.ts";
import { serveBuckets } from "../server/index.ts";
import { BucketKind } from "../declare/bucket.ts";
import { SweepController } from "./sweep.ts";

/** A bucket provider: provisioning, opening and fencing one host's buckets, serving their files and sweeping them. */
export type BucketProvider = Provider<
    typeof BucketKind,
    ReturnType<typeof serveBuckets>,
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

/** Provide a host's buckets under its provider code, one per resource, swept by the host. */
export function bucketProvider(buckets: CatalogueBucketHost): BucketProvider {
    return {
        kind: BucketKind,
        code: buckets.provider,
        object: serveBuckets(buckets),
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
