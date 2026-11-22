import { Digest } from "@destack/schema";
import { boolean, integer, text } from "../table/column.ts";
import { index } from "../table/constraint.ts";
import { defineTable } from "../table/table.ts";

/** The blobs a partition stores: where each one's bytes lie, and what decides its lifetime. */
export const blobTable = defineTable(
    "blob",
    {
        /** The blob's digest. */
        digest: text("digest").validate(Digest).primaryKey(),
        /** The key of its bytes' pack, its own digest for a blob stored alone. */
        pack: text("pack").notNull(),
        /** Where its bytes start in the pack. */
        offset: integer("offset").notNull(),
        /** Its size in bytes. */
        size: integer("size").notNull(),
        /** When its lease ends, in UTC epoch milliseconds, zero once it ended. */
        leasedUntil: integer("leased_until").notNull(),
        /** Whether its last reference may have gone, which a collection checks. */
        isRetired: boolean("is_retired").notNull(),
        /** When a collection found it unreferenced, absent while referenced or unchecked. */
        freedAt: integer("freed_at"),
    },
    {
        constraints: (table) => [
            index("blob_pack").on(table.pack),
            index("blob_leased_until").on(table.leasedUntil),
            index("blob_retired").on(table.isRetired),
        ],
    },
);

/** The tables of a partition's blob store, which its database declares beside its own. */
export const BLOB_TABLES = [blobTable];
