import { check, integer, json, sql, type Column } from "@destack/db";
import { defineSchema, schema } from "@destack/schema";
import { deletionColumns } from "./recoverable.ts";
import type { Trait } from "./trait.ts";

/** A controller's observation of a record. */
export const StatusCondition = defineSchema(
    schema.object({
        /** Whether the condition holds, or has not been established. */
        status: schema.enum(["true", "false", "unknown"]),
        /** The desired generation the controller evaluated. */
        observedGeneration: schema.number().int().min(1),
        /** The machine-readable explanation. */
        reason: schema.string().min(1),
        /** The human-readable explanation. */
        message: schema.string(),
        /** The last change of condition status, in UTC epoch milliseconds. */
        lastTransitionAt: schema.number().int().min(0),
    }),
);
/** A controller's observation of a record. */
export type StatusCondition = schema.Infer<typeof StatusCondition>;

/** Status conditions keyed by their unique domain-specific names. */
export const ConditionMap = defineSchema(schema.record(schema.string().min(1), StatusCondition));

/** Declare the desired generation and a controller's observations. */
export function controlledColumns() {
    return {
        /** The desired state's generation. */
        generation: integer("generation").notNull().default(1),
        /** The latest generation the controller evaluated. */
        observedGeneration: integer("observed_generation").notNull().default(0),
        /** The conditions the controller reports. */
        conditions: json("conditions", ConditionMap)
            .notNull()
            .default(sql`'{}'`),
    };
}

/** Records a controller reconciles. */
export const controlled: Trait<true> & {
    /** Record a controller's observation, keeping the last transition time while the status holds. */
    observe(
        previous: StatusCondition | undefined,
        observation: Omit<StatusCondition, "lastTransitionAt">,
        now: number,
    ): StatusCondition;
} = {
    key: "controlled",
    isDurable: true,
    options: (definition) => (definition.controlled ? true : undefined),
    columns: () => ({ ...controlledColumns(), ...deletionColumns() }),
    constraints: (_options, table, columns) => controlledChecks(table, columns as never),
    methods: () => ({}),
    observe: (previous, observation, now) => ({
        ...observation,
        lastTransitionAt: previous?.status === observation.status ? previous.lastTransitionAt : now,
    }),
};

/** Require valid revisions and generations, and refuse observations of future generations. */
function controlledChecks(
    name: string,
    columns: {
        /** The record revision, including observation changes. */
        readonly revision: Column;
        /** The desired generation. */
        readonly generation: Column;
        /** The observed generation. */
        readonly observedGeneration: Column;
    },
) {
    return [
        check(`${name}_revision`, sql`${columns.revision} >= 1`),
        check(`${name}_generation`, sql`${columns.generation} >= 1`),
        check(
            `${name}_observed_generation`,
            sql`${columns.observedGeneration} BETWEEN 0 AND ${columns.generation}`,
        ),
    ];
}
