import { defineDatabaseSchema } from "@destack/db";
import { tables } from "./tables.ts";
import { relations } from "./relation.ts";

/** Authoritative space administration hosted locally or regionally. */
export const spaceSchema = defineDatabaseSchema({
    name: "destack-space",
    tables,
    relations,
    migrations: new URL("../migration/", import.meta.url),
});
