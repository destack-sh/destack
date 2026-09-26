import { defineTable, TABLE, text } from "../../index.ts";

/** The node columns shared before and after the ancestor index. */
const columns = {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
    parent: text("parent"),
};

/** Application nodes that predate the ancestor index. */
export const baseNode = defineTable("tree_node", columns);

/** The same nodes with one independent parent forest per scope. */
export const node = defineTable("tree_node", columns, {
    tree: { id: "id", scope: "scope", parent: "parent" },
});

/** The ancestor index maintained for the nodes. */
export const tree = node[TABLE].tree!;
