import { defineRelations } from "@destack/db";
import { packageRelations } from "../package/relation.ts";
import { tables } from "./tables.ts";

/** Query relationships in a regional database. */
export const relations = {
    ...defineRelations(tables),
    ...packageRelations,
};
