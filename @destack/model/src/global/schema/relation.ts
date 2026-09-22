import { defineRelations } from "@destack/db";
import { accountRelations } from "../account/relation.ts";
import { directoryRelations } from "../directory/relation.ts";
import { hostRelations } from "../host/relation.ts";
import { routingRelations } from "../routing/relation.ts";
import { tables } from "./tables.ts";

/** Query relationships in the global database. */
export const relations = {
    ...defineRelations(tables),
    ...accountRelations,
    ...directoryRelations,
    ...hostRelations,
    ...routingRelations,
};
