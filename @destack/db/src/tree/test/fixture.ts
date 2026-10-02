import { defineTable, TABLE, text } from "../../index.ts";
import type { Tree } from "../tree.ts";

/** The node columns. */
const columns = {
    id: text("id").primaryKey(),
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
export const tree = requireTree(node[TABLE].tree);

/** Require the tree a table declares. */
function requireTree(declared: Tree | undefined): Tree {
    if (declared === undefined) {
        throw new TypeError("the node table declares no tree");
    }

    return declared;
}
