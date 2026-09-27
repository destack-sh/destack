import { mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { eq, type DatabaseConnection } from "@destack/db";
import type { Provider } from "@destack/resource";
import { identifier } from "@destack/schema";
import type { Bucket } from "../bucket/index.ts";
import { bucket } from "../stack/index.ts";
import { LocalBucket } from "./bucket.ts";

/** Provide buckets as directories under a host directory, one per resource, registered in the space database. */
export function localBucketProvider(
    database: DatabaseConnection,
    directory: string,
): Provider<Bucket> {
    return {
        kind: "bucket",
        code: "local",
        provision: async (resource) => {
            // create the bucket's directory
            const path = join(directory, resource.scope, resource.id);
            await mkdir(path, { recursive: true });

            // register the bucket once; access decides on it by resource
            await database
                .insert(bucket)
                .values({
                    resourceId: identifier("resource").parse(resource.id),
                    scope: identifier("space").parse(resource.scope),
                })
                .onConflictDoNothing();

            return { reference: pathToFileURL(path).href };
        },
        plan: async () => ({ steps: [] }),
        apply: async () => {},
        connect: async (resource) =>
            LocalBucket.open(fileURLToPath(requireReference(resource.reference))),
        destroy: async (resource) => {
            // remove the files, then the bucket's registration
            await rm(fileURLToPath(requireReference(resource.reference)), {
                recursive: true,
                force: true,
            });
            await database
                .delete(bucket)
                .where(eq(bucket.resourceId, identifier("resource").parse(resource.id)));
        },
    };
}

/** Require a provisioned resource reference. */
function requireReference(reference: string | null): string {
    if (reference === null) {
        throw new TypeError("resource is not provisioned");
    }

    return reference;
}
