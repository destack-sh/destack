import { defineRelations } from "@destack/db";
import { packageRelations } from "../package/relation.ts";
import { tables } from "./tables.ts";
import { accessRelations } from "../access/relation.ts";
import { resourceRelations } from "../resource/relation.ts";
import { spaceRelations } from "../space/relation.ts";

/** Query relationships in a regional database. */
export const relations = {
    ...defineRelations(tables),
    ...packageRelations,
    ...accessRelations,
    ...resourceRelations,
    ...spaceRelations,
};
