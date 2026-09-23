import { defineDatabaseSchema } from "@destack/db";
import { tables } from "./tables.ts";
import { relations } from "./relation.ts";

/** Repository and registry records retained in their assigned region. */
export const regionalSchema = defineDatabaseSchema({
    name: "destack-regional",
    tables,
    relations,
    migrations: new URL("../migration/", import.meta.url),
});
