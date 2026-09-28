import { defineTable, TABLE, text } from "../../index.ts";

/** The node columns. */
const columns = {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
    parent: text("parent"),
};

/** Nodes without an ancestor index. */
export const baseNode = defineTable("tree_node", columns);

/** Nodes with one parent forest per scope. */
export const node = defineTable("tree_node", columns, {
    tree: { id: "id", scope: "scope", parent: "parent" },
});

/** The nodes' ancestor index. */
export const tree = node[TABLE].tree!;
