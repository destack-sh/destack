import { fromJsonSchema, present, schema, Uuid } from "@destack/schema";
import { insertEpoch } from "../log/schema.ts";
import { sql } from "../sql/index.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { DatabaseError } from "../error/error.ts";
import { STATE, type TableState, writeState } from "./state.ts";
import { literal, quote } from "../dialect/quote.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { TablePlan, TableStep } from "./plan.ts";

/** Apply a plan in one transaction and record the declared state. */
export async function applyPlan(
    database: DatabaseConnection,
    plan: TablePlan,
    manager: string,
): Promise<void> {
    // skip empty plans
    if (plan.steps.length === 0) {
        return;
    }

    // serialise appliers, defer foreign keys and the triggers, and start the log at a drawn epoch
    const lock =
        plan.dialect === "postgresql"
            ? `SELECT pg_advisory_xact_lock(hashtextextended(${literal(STATE)}, 0))`
            : "PRAGMA defer_foreign_keys = ON";
    const { environment, namespace } = database.state;
    const changes = [
        lock,
        ...plan.before,
        ...plan.steps.filter((step) => step.phase === "tables").flatMap((step) => step.statements),
        ...plan.after,
        insertEpoch(Uuid.v7(environment), namespace),
    ];

    // apply every phase and record the state, checking the references after SQLite rebuilt tables
    const isRebuilt =
        plan.dialect === "sqlite" && plan.steps.some((step) => step.action === "replace");
    const appliedAt = database.state.clock.now();
    await applySteps(database, plan, manager, changes, appliedAt, isRebuilt);
}

/** Apply a plan's phases and record its state. */
async function applySteps(
    database: DatabaseConnection,
    plan: TablePlan,
    manager: string,
    changes: readonly string[],
    appliedAt: number,
    isRebuilt: boolean,
): Promise<void> {
    await database.transaction(async (transaction) => {
        // change the tables, then look for the log again, which the changes may create
        await runStatements(transaction, changes, manager);
        transaction.state.logging = "unknown";

        // convert rows after the triggers
        const pending = plan.steps
            .filter((each) => each.phase === "rows")
            .flatMap((step) => step.statements);

        // record the declared state
        const recorded = plan.state.map((state) =>
            writeState(state, manager, appliedAt, database.state.namespace),
        );
        await runStatements(transaction, [...pending, ...recorded], manager);

        // require every converted value to pass its column's schema, since reads trust stored values
        for (const step of plan.steps) {
            if (step.converted !== undefined) {
                await requireConverted(transaction, step.converted, plan.state, plan.dialect);
            }
        }

        // require valid references after a rebuild
        if (isRebuilt) {
            const violations = await transaction.execute(
                sql`PRAGMA foreign_key_check`,
                schema.looseObject({ table: schema.string() }),
            );
            if (violations.length > 0) {
                const tables = [...new Set(violations.map((row) => row.table))].join(", ");
                throw DatabaseError.of(
                    "MIGRATION_BROKEN",
                    `migration leaves foreign key violations in '${tables}'`,
                );
            }
        }
    });
}

/** Run a manager's statements in order, reporting the one that fails. */
async function runStatements(
    database: DatabaseConnection,
    statements: readonly string[],
    manager: string,
): Promise<void> {
    for (const statement of statements) {
        // wrap the failure in a migration error with the statement's first line
        try {
            await database.executeScript(`${statement};`);
        } catch (cause) {
            const message = cause instanceof Error ? cause.message : String(cause);
            const [line] = statement.split("\n");
            throw DatabaseError.of(
                "MIGRATION_BROKEN",
                `migration of '${manager}' failed at '${line}': ${message}`,
                { cause },
            );
        }
    }
}

/** Require a conversion's written JSON values to pass their columns' recorded schemas. */
async function requireConverted(
    database: DatabaseConnection,
    converted: NonNullable<TableStep["converted"]>,
    states: readonly TableState[],
    dialect: Dialect,
): Promise<void> {
    // find the converted JSON columns in the state the plan records
    const state = present(
        states.find((entry) => entry.table.name === converted.table),
        `the state of ${converted.table}`,
    );
    const columns = state.table.columns.filter(
        (column) => column.kind === "json" && converted.columns.includes(column.name),
    );

    // read the converted values and parse each by its column's recorded JSON Schema
    for (const column of columns) {
        const validator = fromJsonSchema(column.value);
        const rows = await database.execute(
            sql.raw(`SELECT ${quote(column.name)} AS value FROM ${quote(converted.table)}`),
            schema.object({ value: schema.unknown() }),
        );
        for (const { value } of rows) {
            const json: unknown =
                dialect === "sqlite" && typeof value === "string" ? JSON.parse(value) : value;
            const parsed = json === null && column.nullable ? undefined : validator.safeParse(json);
            if (parsed !== undefined && !parsed.success) {
                throw DatabaseError.of(
                    "MIGRATION_BROKEN",
                    `conversion writes a value that '${converted.table}.${column.name}' rejects: '${schema.prettifyError(parsed.error)}'`,
                    { cause: parsed.error },
                );
            }
        }
    }
}
