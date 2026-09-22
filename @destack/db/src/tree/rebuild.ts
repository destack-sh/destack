import { sql } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import { DatabaseError } from "../error/error.ts";
import type { TreeDescription } from "../inspect/tree.ts";

/** The maximum number of ancestor records written in one statement. */
const BATCH_SIZE = 128;

/** Rebuild a historical tree index inside the caller's write transaction. */
export async function rebuildTree(
    database: DatabaseConnection,
    tree: TreeDescription,
): Promise<void> {
    // require atomic replacement of the derived index
    if (!database.connection.transaction) {
        throw new DatabaseError("TRANSACTION_REQUIRED", "tree rebuild requires a transaction");
    }

    // protect the source against concurrent parent changes during reconstruction
    if (database.connection.native.dialect === "postgresql") {
        await database.execute(
            sql`LOCK TABLE ${sql.identifier(tree.table)} IN SHARE ROW EXCLUSIVE MODE`,
        );
    }
    const rows = await database.execute<{ id: string; scope: string; parent: string | null }>(
        sql`SELECT ${sql.identifier(tree.id)} AS id, ${sql.identifier(tree.scope)} AS scope,
            ${sql.identifier(tree.parent)} AS parent FROM ${sql.identifier(tree.table)}`,
    );

    // retain parent records by scope so identical local IDs cannot cross scopes
    const scopes = new Map<string, Map<string, string | null>>();
    for (const row of rows) {
        let parents = scopes.get(row.scope);
        if (!parents) {
            parents = new Map();
            scopes.set(row.scope, parents);
        }
        if (parents.has(row.id)) {
            throw new DatabaseError("INVALID_MIGRATION", "tree identity already exists");
        }
        parents.set(row.id, row.parent);
    }

    // validate every parent chain before replacing the derived index
    for (const parents of scopes.values()) {
        const complete = new Set<string>();
        for (const id of parents.keys()) {
            const path = new Set<string>();
            let current: string | null = id;
            while (current !== null && !complete.has(current)) {
                if (path.has(current)) {
                    throw new DatabaseError("INVALID_MIGRATION", "tree contains a cycle");
                }
                if (!parents.has(current)) {
                    throw new DatabaseError("INVALID_MIGRATION", "tree parent is missing");
                }
                path.add(current);
                current = parents.get(current)!;
            }
            for (const member of path) {
                complete.add(member);
            }
        }
    }

    // write bounded batches without retaining the quadratic ancestor output in memory
    await database.execute(sql`DELETE FROM ${sql.identifier(tree.ancestors)}`);
    let batch: ReturnType<typeof sql>[] = [];
    for (const [scope, parents] of scopes) {
        for (const descendant of parents.keys()) {
            let ancestor: string | null = descendant;
            let depth = 0;
            while (ancestor !== null) {
                batch.push(sql`(${scope}, ${ancestor}, ${descendant}, ${depth})`);
                ancestor = parents.get(ancestor)!;
                depth++;
                if (batch.length === BATCH_SIZE) {
                    await database.execute(sql`INSERT INTO ${sql.identifier(tree.ancestors)}
                        (scope, ancestor, descendant, depth) VALUES ${sql.join(batch, sql`, `)}`);
                    batch = [];
                }
            }
        }
    }
    if (batch.length > 0) {
        await database.execute(sql`INSERT INTO ${sql.identifier(tree.ancestors)}
            (scope, ancestor, descendant, depth) VALUES ${sql.join(batch, sql`, `)}`);
    }
}
