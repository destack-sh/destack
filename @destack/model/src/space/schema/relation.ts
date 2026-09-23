import { defineRelations } from "@destack/db";
import { accessRelations } from "../access/relation.ts";
import { resourceRelations } from "../resource/relation.ts";
import { spaceRelations } from "../space/relation.ts";
import { tables } from "./tables.ts";

/** Query relationships in the authoritative space administration database. */
export const relations = {
    ...defineRelations(tables),
    ...accessRelations,
    ...resourceRelations,
    ...spaceRelations,
};
