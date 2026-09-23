import { defineDatabaseSchema } from "@destack/db";
import { tables } from "./tables.ts";
import { relations } from "./relation.ts";

/** Regional space administration, resources, repositories and registry records. */
export const regionalSchema = defineDatabaseSchema({
    name: "destack-regional",
    tables,
    relations,
    migrations: new URL("../migration/", import.meta.url),
});
