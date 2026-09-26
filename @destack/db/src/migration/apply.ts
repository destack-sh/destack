import { sql } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import { rebuildTree } from "../tree/rebuild.ts";
import { DatabaseError } from "../error/error.ts";
import { STATE, writeState } from "./state.ts";
import type { TablePlan, TableStep } from "./plan.ts";

/** Step kinds applied after triggers are installed, so their row writes reach the log. */
const LOGGED_KINDS: ReadonlySet<TableStep["kind"]> = new Set([
    "convertRows",
    "bridgeColumn",
    "rebuildTree",
]);

/** Apply a plan in one transaction and record the declared state. */
export async function applyPlan(database: DatabaseConnection, plan: TablePlan): Promise<void> {
    // skip empty plans
    if (plan.steps.length === 0) {
        return;
    }

    // serialise appliers, defer foreign keys, and change tables between trigger reinstalls
    const lock =
        plan.dialect === "postgresql"
            ? `SELECT pg_advisory_xact_lock(hashtextextended('${STATE}', 0))`
            : "PRAGMA defer_foreign_keys = ON";
    const changes = [
        lock,
        ...plan.before,
        ...plan.steps
            .filter((step) => !LOGGED_KINDS.has(step.kind))
            .flatMap((step) => step.statements),
        ...plan.after,
    ];

    // rebuild SQLite tables with foreign keys off, which only takes effect outside a transaction
    const isRebuilt =
        plan.dialect === "sqlite" && plan.steps.some((step) => step.kind === "rebuildTable");
    if (isRebuilt) {
        await database.executeScript("PRAGMA foreign_keys = OFF");
    }

    // apply every phase and record the state in one transaction
    const appliedAt = Date.now();
    try {
        await applySteps(database, plan, changes, appliedAt, isRebuilt);
    } finally {
        // restore foreign key enforcement after rebuilding
        if (isRebuilt) {
            await database.executeScript("PRAGMA foreign_keys = ON");
        }
    }
}

/** Apply a plan's phases and record its state in one transaction. */
async function applySteps(
    database: DatabaseConnection,
    plan: TablePlan,
    changes: readonly string[],
    appliedAt: number,
    isRebuilt: boolean,
): Promise<void> {
    await database.transaction(async (transaction) => {
        // change the tables in one script
        await runScript(transaction, changes);

        // convert rows and rebuild trees once the triggers are back, batching statements between trees
        let pending: string[] = [];
        for (const step of plan.steps.filter((entry) => LOGGED_KINDS.has(entry.kind))) {
            // flush the batch, then rebuild the tree through its own queries
            if (step.tree) {
                await runScript(transaction, pending);
                pending = [];
                await rebuildTree(transaction, step.tree).catch((cause: unknown) => {
                    throw new DatabaseError(
                        "MIGRATION_FAILED",
                        `${step.target}: ${step.detail} failed`,
                        {
                            cause,
                        },
                    );
                });
            }
            // batch the step's statements
            else {
                pending.push(...step.statements);
            }
        }

        // record the declared state with the remaining statements
        await runScript(transaction, [
            ...pending,
            ...plan.state.map((state) => writeState(state, appliedAt)),
        ]);

        // require rebuilt tables to leave every reference valid before committing
        if (isRebuilt) {
            const violations = await transaction.execute<{ table: string }>(
                sql`PRAGMA foreign_key_check`,
            );
            if (violations.length > 0) {
                const tables = [...new Set(violations.map((row) => row.table))].join(", ");
                throw new DatabaseError(
                    "MIGRATION_FAILED",
                    `migration leaves foreign key violations in ${tables}`,
                );
            }
        }
    });
}

/** Run statements as one script, reporting the database's failure as a failed migration. */
async function runScript(
    database: DatabaseConnection,
    statements: readonly string[],
): Promise<void> {
    // skip empty phases
    if (statements.length === 0) {
        return;
    }

    // name the database's failure in the migration error
    try {
        await database.executeScript(statements.map((statement) => `${statement};`).join("\n"));
    } catch (cause) {
        const message = cause instanceof Error ? cause.message : String(cause);
        throw new DatabaseError("MIGRATION_FAILED", `migration failed: ${message}`, { cause });
    }
}
