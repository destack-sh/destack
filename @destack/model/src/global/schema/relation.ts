import { defineRelations } from "@destack/db";
import { authenticationRelations } from "../authentication/relation.ts";
import { spaceRelations } from "../space/relation.ts";
import { packageRelations } from "../package/relation.ts";
import { accountRelations } from "../account/relation.ts";
import { hostRelations } from "../host/relation.ts";
import { routingRelations } from "../routing/relation.ts";
import { tables } from "./tables.ts";

/** Query relationships in the global database. */
export const relations = {
    ...defineRelations(tables),
    ...accountRelations,
    ...authenticationRelations,
    ...spaceRelations,
    ...packageRelations,
    ...hostRelations,
    ...routingRelations,
};
