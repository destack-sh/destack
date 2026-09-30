import { Address, type Compare, type Plan, type Step } from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import { defineSchema, type schema } from "@destack/schema";
import { DatabaseKind, type Database } from "../declare/database.ts";
import { planTables } from "../migration/plan.ts";
import { DatabaseState } from "../migration/state.ts";

/** A declared database and its required tables, as the manifest records it. */
export const DatabaseDeclaration = defineSchema(
    DatabaseKind.description.extend(DatabaseState.shape),
);
/** A declared database and its required tables, as the manifest records it. */
export type DatabaseDeclaration = schema.Infer<typeof DatabaseDeclaration>;

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
