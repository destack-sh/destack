import { check, type Column, integer, json, sql } from "@destack/db";
import { schema } from "@destack/schema";
import { Conditions } from "./condition.ts";

/** Define persisted intent revisions and controller observations. */
export function reconcileColumns() {
    return {
        /** The desired configuration generation. */
        generation: integer("generation").notNull().default(1),
        /** The latest generation evaluated, including unsuccessful evaluations. */
        observedGeneration: integer("observed_generation").notNull().default(0),
        /** Conditions reported by the responsible controller. */
        conditions: json("conditions", Conditions).notNull().default(sql`'{}'`),
        /** The requested deletion time; cleanup precedes physical deletion. */
        deletionRequestedAt: integer("deletion_requested_at"),
        /** Controller names whose cleanup must finish before deletion. */
        finalizers: json("finalizers", schema.array(schema.string().min(1))).notNull().default(
            sql`'[]'`,
        ),
    };
}

/** Require valid revisions and prevent observations of future generations. */
export function reconcileChecks(name: string, columns: {
    /** The record revision, including observed-state changes. */
    revision: Column;
    /** The desired generation. */
    generation: Column;
    /** The observed generation. */
    observedGeneration: Column;
}) {
    return [
        check(`${name}_revision`, sql`${columns.revision} >= 1`),
        check(`${name}_generation`, sql`${columns.generation} >= 1`),
        check(
            `${name}_observed_generation`,
            sql`${columns.observedGeneration} BETWEEN 0 AND ${columns.generation}`,
        ),
    ];
}
