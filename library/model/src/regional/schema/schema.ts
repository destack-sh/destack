import { defineDatabaseSchema } from "@destack/db";
import { tables } from "./tables.ts";
import { relations } from "./relation.ts";

/** Regional spaces, resources, packages, and access records. */
export const regionalSchema = defineDatabaseSchema({
    name: "destack-regional",
    tables,
    relations,
    migrations: new URL("../migration/", import.meta.url),
});
