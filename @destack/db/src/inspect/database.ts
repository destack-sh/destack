import { Address, type Comparator, type Plan, type Step } from "@destack/resource";
import { ResourceError, unreachablePlan } from "@destack/resource/error";
import { graph } from "@destack/package";
import { schema } from "@destack/schema";
import { type Database, DatabaseDescription } from "../declare/database.ts";
import { changedTables, planTables } from "../migration/plan.ts";

/** A table's state in one dialect, as a database declaration records it. */
type TableState = DatabaseDescription["tables"][keyof DatabaseDescription["tables"]][number];

/** Describe a declared database. */
export function describeDatabase(database: Database): DatabaseDescription {
    return DatabaseDescription.parse({
        name: database.name,
        kind: database.kind,
        spec: database.spec,
        ...database.state(),
    });
}

/** Plan a database's table changes between releases in every dialect, the steps both share once. */
export const compareDatabase: Comparator = (before, after) => {
    // read both releases' databases, whose table states record their releases
    const earlier = DatabaseDescription.parse(before.description);
    const later = DatabaseDescription.parse(after.description);

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
                changed: changedTables(applied, later.tables[dialect]),
                empty: new Set(),
            });
        } catch (error) {
            throw ResourceError.is(error, "PLAN_UNREACHABLE")
                ? unreachablePlan(
                      error.data.problems.map((problem) => ({
                          ...problem,
                          target: Address.join(database, problem.target),
                      })),
                  )
                : error;
        }

        // list each step once across dialects
        for (const { action, risk, target, detail } of plan.steps) {
            const addressed = Address.join(database, target);
            const key = JSON.stringify([action, addressed, detail]);
            steps.set(key, { action, target: addressed, risk, detail });
        }
    }

    return { steps: [...steps.values()] };
};

/** List a database's tables as its member symbols, each described by its state in every dialect. */
export function databaseSymbols(declared: Database): graph.MemberSymbol[] {
    // collect each table's state by dialect
    const tables = new Map<string, Record<string, TableState>>();
    for (const [dialect, states] of Object.entries(declared.state().tables)) {
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
