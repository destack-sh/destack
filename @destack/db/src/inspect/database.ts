import type { Compare, Step } from "@destack/resource";
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

    // plan the earlier release's tables to the later release's in each dialect
    const steps = new Map<string, Step>();
    for (const dialect of ["sqlite", "postgresql"] as const) {
        const applied = earlier.tables[dialect];
        const plan = planTables({
            applied,
            existing: applied.map((state) => state.table.name),
            declared: later.tables[dialect],
            dialect,
        });

        // keep each step once across dialects
        for (const { kind, risk, target, detail } of plan.steps) {
            steps.set(JSON.stringify([kind, target, detail]), { kind, risk, target, detail });
        }
    }

    return { steps: [...steps.values()] };
};
