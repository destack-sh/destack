import { defineTable } from "../table/table.ts";
import { index } from "../table/constraint.ts";
import { integer, text } from "../table/column.ts";
import { Digest } from "@destack/schema";

/** The blobs writers lease until a moment: a write in flight, a blob no row references yet. */
export const blobLease = defineTable(
    "blob_lease",
    {
        /** The leased blob. */
        digest: text("digest").validate(Digest).primaryKey(),
        /** When the lease ends, in UTC epoch milliseconds. */
        until: integer("until").notNull(),
    },
    { constraints: (table) => [index("blob_lease_until").on(table.until)] },
);

/** The blobs whose last reference may have gone, deleted unless a row or a lease keeps them. */
export const blobGarbage = defineTable("blob_garbage", {
    /** The blob. */
    digest: text("digest").validate(Digest).primaryKey(),
});

/** The tables of a partition's blob store, which its database declares beside its own. */
export const BLOB_TABLES = [blobLease, blobGarbage];
