import { schema } from "@destack/schema";
import { sql, type SQL } from "../sql/index.ts";
import { defineTable, TABLE, Table } from "../table/table.ts";
import { text, integer } from "../table/column.ts";
import { index } from "../table/constraint.ts";
import { assertNever, DatabaseError } from "../error/index.ts";
import type { TreeDescription } from "../inspect/tree.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Snapshot } from "../log/snapshot.ts";

/** One ancestor of a node, with its distance. */
const ANCESTRY = schema.looseObject({
    /** The node. */
    descendant: schema.string(),
    /** The ancestor. */
    ancestor: schema.string(),
    /** The distance, 0 for the node itself. */
    depth: schema.number(),
});

/** A scoped parent relationship with an ancestor index. */
export class Tree {
    /** The tree's columns. */
    readonly definition: TreeDefinition;
    /** The ancestor index table, with each node at depth zero. */
    readonly ancestors;
    /** The revision table serializing hierarchy writes per scope. */
    readonly revision;

    /** Create the tree of a table's columns. */
    constructor(definition: TreeDefinition) {
        // copy the columns
        this.definition = Object.freeze({ ...definition });

        // require text identities and an optional parent
        for (const name of [definition.id, definition.scope, definition.parent]) {
            const column = definition.table[TABLE].columns[name];
            if (column === undefined || column.definition.kind !== "text") {
                throw new DatabaseError("INVALID_MIGRATION", `unknown tree column: ${name}`);
            }
            if (column.definition.nullable !== (name === definition.parent)) {
                throw new DatabaseError(
                    "INVALID_MIGRATION",
                    `invalid tree column nullability: ${name}`,
                );
            }
        }

        // index ancestors and descendants per scope
        const name = `${definition.table[TABLE].name}_${definition.name}_ancestor`;
        const owner = { package: definition.table[TABLE].package };
        this.ancestors = defineTable(
            name,
            {
                scope: text("scope").primaryKey(),
                ancestor: text("ancestor").primaryKey(),
                descendant: text("descendant").primaryKey(),
                depth: integer("depth").notNull(),
            },
            {
                constraints: (path) => [
                    index(`${name}_descendant`).on(path.scope, path.descendant, path.ancestor),
                ],
                // log each path under its scope
                log: { retention: "window" },
            },
            owner,
        );

        // declare the revision table
        this.revision = defineTable(
            `${name}_revision`,
            { scope: text("scope").primaryKey(), revision: integer("revision").notNull() },
            {},
            owner,
        );
    }

    /** Read some nodes' proper ancestors through the index, nearest first. */
    async ancestry(
        snapshot: Snapshot,
        scope: string,
        ids: readonly string[],
    ): Promise<{ readonly descendant: string; readonly ancestor: string }[]> {
        const paths = await snapshot.select(
            this.ancestors,
            ["scope", "descendant"],
            ids.map((id) => [scope, id]),
        );

        return paths
            .map((path) => ANCESTRY.parse(path))
            .filter((path) => path.depth > 0)
            .toSorted((left, right) => left.depth - right.depth)
            .map(({ descendant, ancestor }) => ({ descendant, ancestor }));
    }

    /** Describe the tree's physical columns. */
    describe(): TreeDescription {
        const { table, name, id, scope, parent } = this.definition;

        return {
            name,
            table: table[TABLE].sqlName,
            id: table[TABLE].column(id).definition.name,
            scope: table[TABLE].column(scope).definition.name,
            parent: table[TABLE].column(parent).definition.name,
            ancestors: this.ancestors[TABLE].sqlName,
            revision: this.revision[TABLE].sqlName,
        };
    }

    /** Select roots within one scope. */
    roots(scope: string): SQL {
        const table = this.definition.table[TABLE];

        return sql`${table.column(this.definition.scope)} = ${scope} AND ${table.column(this.definition.parent)} IS NULL`;
    }

    /** Select immediate children within one scope. */
    children(scope: string, parent: string): SQL {
        const table = this.definition.table[TABLE];

        return sql`${table.column(this.definition.scope)} = ${scope} AND ${table.column(this.definition.parent)} = ${parent}`;
    }

    /** Select strict ancestors of a node within one scope. */
    ancestorsOf(scope: string, descendant: string): SQL {
        const table = this.definition.table[TABLE];
        const path = this.ancestors;

        return sql`${table.column(this.definition.scope)} = ${scope} AND EXISTS (
            SELECT 1 FROM ${path} WHERE ${path.scope} = ${scope}
            AND ${path.descendant} = ${descendant} AND ${path.ancestor} = ${table.column(this.definition.id)}
            AND ${path.depth} > 0
        )`;
    }

    /** Select strict descendants of a node within one scope. */
    descendantsOf(scope: string, ancestor: string): SQL {
        const table = this.definition.table[TABLE];
        const path = this.ancestors;

        return sql`${table.column(this.definition.scope)} = ${scope} AND EXISTS (
            SELECT 1 FROM ${path} WHERE ${path.scope} = ${scope}
            AND ${path.ancestor} = ${ancestor} AND ${path.descendant} = ${table.column(this.definition.id)}
            AND ${path.depth} > 0
        )`;
    }

    /** Move a subtree or make it a root and refuse cycles. */
    async move(
        scope: string,
        id: string,
        parent: string | null,
        database: DatabaseConnection,
    ): Promise<void> {
        // update the parent of an existing node
        const tree = this.describe();
        const rows = await database.execute(sql`UPDATE ${sql.identifier(tree.table)}
            SET ${sql.identifier(tree.parent)} = ${parent}
            WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.id)} = ${id}
            RETURNING ${sql.identifier(tree.id)}`);
        if (rows.length === 0) {
            throw new DatabaseError("TREE_NOT_FOUND", "tree node not found");
        }
    }

    /** Remove a node with a policy for its children. */
    async remove(
        scope: string,
        id: string,
        children: TreeDeletion,
        database: DatabaseConnection,
    ): Promise<void> {
        const tree = this.describe();
        await database.transaction(
            async (transaction) => {
                // read the parent first
                const [node] = await transaction.execute(
                    sql`SELECT ${sql.identifier(tree.parent)} AS parent FROM ${sql.identifier(tree.table)}
                        WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.id)} = ${id}`,
                    schema.object({ parent: schema.string().nullable() }),
                );
                if (node === undefined) {
                    throw new DatabaseError("TREE_NOT_FOUND", "tree node not found");
                }

                // move the children to the parent
                if (children === "reparent") {
                    await transaction.execute(sql`UPDATE ${sql.identifier(tree.table)}
                    SET ${sql.identifier(tree.parent)} = ${node.parent}
                    WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.parent)} = ${id}`);
                }
                // delete the strict descendants in one statement
                else if (children === "subtree") {
                    await transaction.execute(sql`DELETE FROM ${sql.identifier(tree.table)}
                    WHERE ${sql.identifier(tree.scope)} = ${scope} AND ${sql.identifier(tree.id)} IN (
                        SELECT descendant FROM ${sql.identifier(tree.ancestors)}
                        WHERE scope = ${scope} AND ancestor = ${id} AND depth > 0
                    )`);
                }
                // leave rejection to the constraint
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

/** How a removal treats the node's children. */
export type TreeDeletion = "restrict" | "subtree" | "reparent";

/** The columns of one single-parent tree per scope. */
export interface TreeDefinition {
    /** The declaration name. */
    readonly name: string;
    /** The application table. */
    readonly table: Table;
    /** The node identity property. */
    readonly id: string;
    /** The tree scope property. */
    readonly scope: string;
    /** The nullable parent property. */
    readonly parent: string;
}

/** Add each tree's ancestor and revision tables beside its table. */
export function expandTrees(tables: readonly Table[]): Table[] {
    return [
        ...new Set(
            tables.flatMap((table) => {
                const tree = table[TABLE].tree;

                return tree === undefined ? [table] : [table, tree.ancestors, tree.revision];
            }),
        ),
    ];
}
