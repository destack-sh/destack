import type { Table } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { spaceDatabase } from "@destack/space/stack";
import { bucket } from "./bucket.ts";

/** The buckets of the spaces a region holds. */
export const bucketTables: readonly Table[] = [bucket];

/** The regional database holding buckets beside their spaces. */
export const bucketDatabase = defineDatabase({
    name: "regional",
    tables: [...bucketTables, ...spaceDatabase.tables],
});
