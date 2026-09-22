import type { DatabaseConnection } from "@destack/db";
import { rebuildTree } from "@destack/db/tree";

/** Populate the ancestor indexes declared by this migration. */
export async function migrate(database: DatabaseConnection): Promise<void> {
    await rebuildTree(database, {
        name: "parent",
        table: "example_item",
        id: "id",
        scope: "scope",
        parent: "parent",
        ancestors: "example_item_parent_ancestor",
        revision: "example_item_parent_ancestor_revision",
    });
}
