import { and, type DatabaseConnection, eq } from "@destack/db";
import { schema } from "@destack/schema";
import type { CatalogueBucketHost } from "@destack/bucket/provider";
import { bucket } from "../../object/index.ts";
import { S3Server } from "@destack/bucket/s3";

/** Serve a machine's buckets over S3. */
export function serveS3(host: CatalogueBucketHost, database: DatabaseConnection): S3Server {
    return new S3Server({
        region: host.region,
        // read the credentials of the space an access key names
        credentials: async (accessKeyId) => {
            const space = schema.identifier("space").safeParse(accessKeyId);

            return space.success ? host.credentials(space.data) : undefined;
        },
        open: async (name, accessKeyId) => {
            // refuse a name no bucket takes and a key no space signs with
            const bucketId = schema.identifier("bucket").safeParse(name);
            const space = schema.identifier("space").safeParse(accessKeyId);
            if (!bucketId.success || !space.success) {
                return undefined;
            }

            // open a bucket of the signing space, refusing another space's
            const [found] = await database
                .select({ id: bucket.table.id })
                .from(bucket.table)
                .where(and(eq(bucket.table.id, bucketId.data), eq(bucket.table.scope, space.data)));

            return found === undefined ? undefined : host.named(name);
        },
    });
}
