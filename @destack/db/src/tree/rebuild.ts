import { sql, type SQL } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import { PARAMETER_BUDGET } from "../dialect/dialect.ts";
import { DatabaseError } from "../error/error.ts";
import type { TreeDescription } from "../inspect/tree.ts";

/** The bound parameters of one ancestor record: scope, ancestor, descendant and depth. */
const RECORD_PARAMETERS = 4;

/** The most ancestor records one statement writes. */
const BATCH_SIZE = Math.floor(PARAMETER_BUDGET / RECORD_PARAMETERS);

/** Rebuild a historical tree index inside the caller's write transaction. */
export async function rebuildTree(
    database: DatabaseConnection,
    tree: TreeDescription,
): Promise<void> {
    // require atomic replacement of the derived index
    if (!database.driver.transaction) {
        throw new DatabaseError("TRANSACTION_REQUIRED", "tree rebuild requires a transaction");
    }

    // protect the source against concurrent parent changes during reconstruction
    if (database.dialect === "postgresql") {
        await database.execute(
            sql`LOCK TABLE ${sql.identifier(tree.table)} IN SHARE ROW EXCLUSIVE MODE`,
        );
    }

    // read every parent link
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
    const flush = async (batch: readonly SQL[]) =>
        database.execute(sql`INSERT INTO ${sql.identifier(tree.ancestors)}
            (scope, ancestor, descendant, depth) VALUES ${sql.join([...batch], sql`, `)}`);
    let batch: SQL[] = [];
    for (const [scope, parents] of scopes) {
        for (const descendant of parents.keys()) {
            // walk each node's chain up to its root
            let ancestor: string | null = descendant;
            let depth = 0;
            while (ancestor !== null) {
                batch.push(sql`(${scope}, ${ancestor}, ${descendant}, ${depth})`);
                ancestor = parents.get(ancestor)!;
                depth++;
                if (batch.length === BATCH_SIZE) {
                    await flush(batch);
                    batch = [];
                }
            }
        }
    }

    // write the last partial batch
    if (batch.length > 0) {
        await flush(batch);
    }
}
