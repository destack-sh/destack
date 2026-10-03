import { Address, type Compare, type Plan, type Step } from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import { graph } from "@destack/package";
import { defineSchema, schema, type JsonValue } from "@destack/schema";
import { DatabaseKind, type Database } from "../declare/database.ts";
import { planTables } from "../migration/plan.ts";
import { DatabaseState } from "../migration/state.ts";

/** A declared database and its required tables, as the manifest records it. */
export const DatabaseDeclaration = defineSchema(
    DatabaseKind.description.extend(DatabaseState.shape),
);
/** A declared database and its required tables, as the manifest records it. */
export type DatabaseDeclaration = schema.Infer<typeof DatabaseDeclaration>;

/** A table's state in one dialect, as a database declaration records it. */
type TableState = DatabaseDeclaration["tables"][keyof DatabaseDeclaration["tables"]][number];

/** Describe a declared database. */
export function describeDatabase(database: Database): DatabaseDeclaration {
    return DatabaseDeclaration.parse({
        name: database.name,
        kind: database.kind,
        spec: database.spec,
        ...database.state(),
    });
}

/** Plan a database's table changes between releases in every dialect, the steps both share once. */
export const compareDatabase: Compare = (before, after) => {
    // read both releases' databases, whose table states carry their own releases
    const earlier = DatabaseDeclaration.parse(before.description);
    const later = DatabaseDeclaration.parse(after.description);

    // plan the tables in each dialect under the database's address
    const database = Address.join("database", later.name);
    const steps = new Map<string, Step>();
    for (const dialect of ["sqlite", "postgresql"] as const) {
        const applied = earlier.tables[dialect];
        let plan: Plan;
        try {
            plan = planTables({
                applied,
                existing: applied.map((state) => state.table.name),
                declared: later.tables[dialect],
                dialect,
            });
        } catch (error) {
            throw error instanceof PlanError
                ? new PlanError(
                      error.problems.map((problem) => ({
                          ...problem,
                          target: Address.join(database, problem.target),
                      })),
                  )
                : error;
        }

        // keep each step once across dialects
        for (const { action, risk, target, detail } of plan.steps) {
            const addressed = Address.join(database, target);
            const key = JSON.stringify([action, addressed, detail]);
            steps.set(key, { action, target: addressed, risk, detail });
        }
    }

    return { steps: [...steps.values()] };
};

/** List a database's tables as its member symbols, each described by its state in every dialect. */
export function databaseSymbols(input: Record<string, JsonValue>): graph.MemberSymbol[] {
    // collect each table's state by dialect
    const database = DatabaseDeclaration.parse(input);
    const tables = new Map<string, Record<string, TableState>>();
    for (const [dialect, states] of Object.entries(database.tables)) {
        for (const state of states) {
            tables.set(state.table.name, { ...tables.get(state.table.name), [dialect]: state });
        }
    }

    return schema.array(graph.MemberSymbol).parse(
        [...tables].map(([name, description]) => ({
            member: { kind: "table", name, description },
            relationships: [],
        })),
    );
}
