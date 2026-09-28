import { sql, type SQL } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import { PARAMETER_BUDGET } from "../dialect/dialect.ts";
import { DatabaseError } from "../error/error.ts";
import type { TreeDescription } from "../inspect/tree.ts";

/** The bound parameters of one ancestor record. */
const RECORD_PARAMETERS = 4;

/** The most ancestor records one statement writes. */
const BATCH_SIZE = Math.floor(PARAMETER_BUDGET / RECORD_PARAMETERS);

/** The temporary staging table of a rebuild. */
const STAGED = "destack_tree_rebuild";

/** Rebuild a tree index in the caller's write transaction. */
export async function rebuildTree(
    database: DatabaseConnection,
    tree: TreeDescription,
): Promise<void> {
    // require a transaction
    if (!database.driver.transaction) {
        throw new DatabaseError("TRANSACTION_REQUIRED", "tree rebuild requires a transaction");
    }

    // lock the source against parent changes
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

    // group parents by scope
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

    // validate every parent chain
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

    // stage the paths in bounded batches
    const staged = sql.identifier(STAGED);
    await database.execute(sql`CREATE TEMPORARY TABLE ${staged}
        (scope text NOT NULL, ancestor text NOT NULL, descendant text NOT NULL, depth integer NOT NULL)`);
    const flush = async (batch: readonly SQL[]) =>
        database.execute(sql`INSERT INTO ${staged}
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

    // stage the last batch
    if (batch.length > 0) {
        await flush(batch);
    }

    // remove stale paths and add missing ones
    const ancestors = sql.identifier(tree.ancestors);
    const same = (left: typeof staged, right: typeof staged) => sql`${left}.scope = ${right}.scope
        AND ${left}.ancestor = ${right}.ancestor
        AND ${left}.descendant = ${right}.descendant
        AND ${left}.depth = ${right}.depth`;
    await database.execute(sql`DELETE FROM ${ancestors}
        WHERE NOT EXISTS (SELECT 1 FROM ${staged} WHERE ${same(staged, ancestors)})`);
    await database.execute(sql`INSERT INTO ${ancestors} (scope, ancestor, descendant, depth)
        SELECT scope, ancestor, descendant, depth FROM ${staged}
        WHERE NOT EXISTS (SELECT 1 FROM ${ancestors} WHERE ${same(ancestors, staged)})`);
    await database.execute(sql`DROP TABLE ${staged}`);
}
