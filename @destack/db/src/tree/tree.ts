import { table, TABLE, type Table } from "../table/table.ts";
import { text, integer } from "../table/column.ts";
import { index, unique } from "../table/constraint.ts";
import { assertNever, DatabaseError } from "../error/index.ts";
import type { TreeDescription } from "../inspect/tree.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { sql, type SQL } from "drizzle-orm";

/** A scoped parent relationship with a transactionally maintained ancestor index. */
export class Tree {
    /** Immutable application column selection used by migrations and inspection. */
    readonly definition: TreeDefinition;
    /** Indexed ancestry, including each node at depth zero. */
    readonly ancestors;
    /** Serialize concurrent hierarchy writes within one scope. */
    readonly revision;

    /** Describe an existing application table's tree columns. */
    constructor(definition: TreeDefinition) {
        // retain the selected columns independently of the caller's declaration object
        this.definition = Object.freeze({ ...definition });

        // require text identities and an optional parent in an existing application table
        for (const name of [definition.id, definition.scope, definition.parent]) {
            const column = definition.table[TABLE].columns[name];
            if (
                !Object.hasOwn(definition.table[TABLE].columns, name) ||
                column.definition.kind !== "text"
            ) {
                throw new DatabaseError("INVALID_MIGRATION", `unknown tree column: ${name}`);
            }
            if (column.definition.nullable !== (name === definition.parent)) {
                throw new DatabaseError(
                    "INVALID_MIGRATION",
                    `invalid tree column nullability: ${name}`,
                );
            }
        }

        // index both ancestor membership and descendant lookup within each scope
        const name = `${definition.table[TABLE].name}_${definition.name}_ancestor`;
        this.ancestors = table(
            name,
            {
                scope: text("scope").notNull(),
                ancestor: text("ancestor").notNull(),
                descendant: text("descendant").notNull(),
                depth: integer("depth").notNull(),
            },
            (path) => [
                unique(`${name}_path`).on(path.scope, path.ancestor, path.descendant),
                index(`${name}_descendant`).on(path.scope, path.descendant, path.ancestor),
            ],
        );

        // retain the lock row in the ordinary schema and migration snapshots
        this.revision = table(`${name}_revision`, {
            scope: text("scope").primaryKey(),
            revision: integer("revision").notNull(),
        });
    }

    /** Describe the physical columns used by generated maintenance SQL. */
    describe(): TreeDescription {
        const { table, name, id, scope, parent } = this.definition;
        const columns = table[TABLE].columns;

        return {
            name,
            table: table[TABLE].name,
            id: columns[id].definition.name,
            scope: columns[scope].definition.name,
            parent: columns[parent].definition.name,
            ancestors: this.ancestors[TABLE].name,
            revision: this.revision[TABLE].name,
        };
    }

    /** Select roots within one scope. */
    roots(scope: string): SQL {
        const columns = this.definition.table[TABLE].columns;

        return sql`${columns[this.definition.scope]} = ${scope} AND ${columns[this.definition.parent]} IS NULL`;
    }

    /** Select immediate children within one scope. */
    children(scope: string, parent: string): SQL {
        const columns = this.definition.table[TABLE].columns;

        return sql`${columns[this.definition.scope]} = ${scope} AND ${columns[this.definition.parent]} = ${parent}`;
    }

    /** Select strict ancestors of a node within one scope. */
    ancestorsOf(scope: string, descendant: string): SQL {
        const columns = this.definition.table[TABLE].columns;
        const path = this.ancestors;

        return sql`${columns[this.definition.scope]} = ${scope} AND EXISTS (
            SELECT 1 FROM ${path} WHERE ${path.scope} = ${scope}
            AND ${path.descendant} = ${descendant} AND ${path.ancestor} = ${columns[this.definition.id]}
            AND ${path.depth} > 0
        )`;
    }

    /** Select strict descendants of a node within one scope. */
    descendantsOf(scope: string, ancestor: string): SQL {
        const columns = this.definition.table[TABLE].columns;
        const path = this.ancestors;

        return sql`${columns[this.definition.scope]} = ${scope} AND EXISTS (
            SELECT 1 FROM ${path} WHERE ${path.scope} = ${scope}
            AND ${path.ancestor} = ${ancestor} AND ${path.descendant} = ${columns[this.definition.id]}
            AND ${path.depth} > 0
        )`;
    }

    /** Move a subtree, or make it a root, with engine-enforced cycle protection. */
    async move(
        scope: string,
        id: string,
        parent: string | null,
        database: DatabaseConnection,
    ): Promise<void> {
        const tree = this.describe();
        const rows = await database.execute(sql`UPDATE ${sql.identifier(tree.table)}
            SET ${sql.identifier(tree.parent)} = ${parent}
            WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.id)} = ${id}
            RETURNING ${sql.identifier(tree.id)}`);
        if (rows.length === 0) {
            throw new DatabaseError("TREE_NOT_FOUND", "tree node not found");
        }
    }

    /** Remove a node with an explicit policy for its children. */
    async remove(
        scope: string,
        id: string,
        children: TreeDeletion,
        database: DatabaseConnection,
    ): Promise<void> {
        const tree = this.describe();
        await database.transaction(
            async (transaction) => {
                // retain the parent's identity before reparenting children or deleting descendants
                const rows = await transaction.execute<{ parent: string | null }>(sql`
                SELECT ${sql.identifier(tree.parent)} AS parent FROM ${sql.identifier(tree.table)}
                WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.id)} = ${id}`);
                if (rows.length === 0) {
                    throw new DatabaseError("TREE_NOT_FOUND", "tree node not found");
                }

                // move direct children to the deleted node's parent
                if (children === "reparent") {
                    await transaction.execute(sql`UPDATE ${sql.identifier(tree.table)}
                    SET ${sql.identifier(tree.parent)} = ${rows[0].parent}
                    WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.parent)} = ${id}`);
                }
                // delete deepest descendants first so every deletion preserves the parent invariant
                else if (children === "subtree") {
                    const descendants = await transaction.execute<{ id: string }>(sql`
                    SELECT descendant AS id FROM ${sql.identifier(tree.ancestors)}
                    WHERE scope = ${scope} AND ancestor = ${id} AND depth > 0 ORDER BY depth DESC`);
                    for (const descendant of descendants) {
                        await transaction.execute(sql`DELETE FROM ${sql.identifier(tree.table)}
                        WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.id)} = ${descendant.id}`);
                    }
                }
                // leave child rejection to the same constraint that protects ordinary SQL writes
                else if (children !== "restrict") {
                    assertNever(children);
                }

                // delete the node
                await transaction.execute(sql`DELETE FROM ${sql.identifier(tree.table)}
                WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.id)} = ${id}`);
            },
            { isolationLevel: "serializable" },
        );
    }
}

/** The treatment of children when removing a tree node. */
export type TreeDeletion = "restrict" | "subtree" | "reparent";

/** Columns defining one single-parent tree within each scope. */
export interface TreeDefinition {
    /** Stable declaration name used in generated SQL identifiers. */
    readonly name: string;
    /** Existing application table. */
    readonly table: Table;
    /** The application property containing the node identity. */
    readonly id: string;
    /** The application property containing the tree scope. */
    readonly scope: string;
    /** The application property containing the nullable parent identity. */
    readonly parent: string;
}

/** Declare a tree and its ancestor table without opening a database. */
export function defineTree(definition: TreeDefinition): Tree {
    return new Tree(definition);
}
