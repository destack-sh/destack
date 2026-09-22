import { defineDatabaseSchema, table, text } from "../../index.ts";
import { defineTree } from "../tree.ts";

/** Application nodes that predate the ancestor index. */
export const node = table("tree_node", {
    id: text("id").primaryKey().notNull(),
    scope: text("scope").notNull(),
    parent: text("parent"),
});

/** One independent parent forest per scope. */
export const tree = defineTree({
    name: "parent",
    table: node,
    id: "id",
    scope: "scope",
    parent: "parent",
});

/** The original application schema without derived ancestry. */
export const baseSchema = defineDatabaseSchema({
    name: "tree-example",
    tables: { node },
    migrations: new URL("./migration/base/", import.meta.url),
});

/** The application schema after adding an ancestor index. */
export const treeSchema = defineDatabaseSchema({
    ...baseSchema,
    tables: { node, ancestors: tree.ancestors, revision: tree.revision },
    trees: [tree],
    migrations: new URL("./migration/tree/", import.meta.url),
});
