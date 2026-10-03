import { pathToFileURL } from "node:url";
import type { DatabaseHandle } from "@destack/db/blob";
import { type Open, type Provider, type Provision } from "@destack/resource";
import { schema } from "@destack/schema";
import type { LocalBucketHost } from "../local/index.ts";
import { serveBucket } from "../server/index.ts";
import { BucketKind } from "../declare/bucket.ts";

/** Provide buckets as directories of a host's local buckets, one per resource, serving their files from the host. */
export function localBucketProvider(buckets: LocalBucketHost): Provider<
    typeof BucketKind,
    ReturnType<typeof serveBucket>,
    DatabaseHandle
> & {
    readonly provision: Provision<typeof BucketKind>;
    readonly open: Open<typeof BucketKind, DatabaseHandle>;
} {
    return {
        kind: BucketKind,
        code: "local",
        object: serveBucket(buckets),
        provision: {
            provision: async (resource) => {
                // create the bucket's directory in its space
                const opened = { bucketId: schema.identifier("bucket").parse(resource.id) };
                await buckets.open(opened, resource.scope);

                return { reference: pathToFileURL(buckets.path(opened)).href };
            },
            destroy: (resource) =>
                buckets.destroy({ bucketId: schema.identifier("bucket").parse(resource.id) }),
        },
        open: {
            open: async (resource) => {
                // open the bucket's catalogue and blobs
                const opened = await buckets.open({
                    bucketId: schema.identifier("bucket").parse(resource.id),
                });

                return opened.database();
            },
        },
    };
}
