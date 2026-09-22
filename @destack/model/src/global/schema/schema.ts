import { defineDatabaseSchema } from "@destack/db";
import { tables } from "./tables.ts";
import { relations } from "./relation.ts";

/** Global account, authentication, host, and directory records. */
export const globalSchema = defineDatabaseSchema({
    name: "destack-global",
    tables,
    relations,
    migrations: new URL("../migration/", import.meta.url),
});
