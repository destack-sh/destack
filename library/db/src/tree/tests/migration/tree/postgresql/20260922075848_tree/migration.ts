import type { DatabaseConnection } from "@destack/db";
import { rebuildTree } from "@destack/db/tree";

/** Populate the ancestor indexes declared by this migration. */
export async function migrate(database: DatabaseConnection): Promise<void> {
    await rebuildTree(database, {
        name: "parent",
        table: "tree_node",
        id: "id",
        scope: "scope",
        parent: "parent",
        ancestors: "tree_node_parent_ancestor",
        revision: "tree_node_parent_ancestor_revision",
    });
}
