import { and, type DatabaseConnection, eq } from "@destack/db";
import { schema } from "@destack/schema";
import type { CatalogueBucketHost } from "../catalogue/index.ts";
import { bucket } from "../object/index.ts";
import { S3Server } from "../s3/index.ts";

/** Serve a cell's buckets over S3: each space signs with its own credentials and reaches only the buckets the bucket service keeps in it. */
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
            const [kept] = await database
                .select({ id: bucket.table.id })
                .from(bucket.table)
                .where(and(eq(bucket.table.id, bucketId.data), eq(bucket.table.scope, space.data)));

            return kept === undefined ? undefined : host.named(name);
        },
    });
}
