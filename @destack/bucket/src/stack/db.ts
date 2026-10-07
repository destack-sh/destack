import { journal } from "@destack/audit/stack";
import { defineDatabase, type Table } from "@destack/db";
import { space } from "@destack/space/object";
import { bucket } from "../object/index.ts";

/** The bucket service's tables: its buckets and its journal. */
export const bucketTables: readonly Table[] = [...bucket.tables, journal];

/** The bucket service's database, with copies of the spaces its buckets live in. */
export const bucketDatabase = defineDatabase({
    name: "bucket",
    tables: bucketTables,
    copies: [space.table],
});
