import { check, decodeRow, integer, json, sql, type Column, type Table } from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { defineSchema, schema } from "@destack/schema";
import { method } from "../method/method.ts";
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

/** A controller's report on one record: the generation it evaluated, its conditions and observed fields. */
export const Observation = defineSchema(
    schema.object({
        /** The desired generation the controller evaluated. */
        observedGeneration: schema.number().int().min(0),
        /** The conditions by name, their transition times kept while their status holds. */
        conditions: schema.record(
            schema.string().min(1),
            StatusCondition.omit({ observedGeneration: true, lastTransitionAt: true }),
        ),
        /** The observed fields the controller writes. */
        fields: schema.record(schema.string(), schema.json()).optional(),
    }),
);
/** A controller's report on one record. */
export type Observation = schema.Infer<typeof Observation>;

/** Record a controller's observation of its target at its generation. */
const observe = method({ permission: null, isSystem: true, input: Observation }).handle(
    async (call) => {
        // merge the conditions and keep transition times while their status holds
        const target = call.target as {
            readonly conditions: Readonly<Record<string, StatusCondition>>;
        };
        const { observedGeneration, conditions, fields } = call.input as Observation;
        const merged = Object.fromEntries(
            Object.entries(conditions).map(([name, condition]) => [
                name,
                controlled.observe(
                    target.conditions[name],
                    { ...condition, observedGeneration },
                    call.now,
                ),
            ]),
        );

        // write the observed state
        return call.observe({
            ...decodeRow(call.object.table as Table, fields ?? {}),
            observedGeneration,
            conditions: { ...target.conditions, ...merged },
        });
    },
);

/** Remove a record with a requested deletion after its controller finishes. */
const finalize = method({
    permission: null,
    isSystem: true,
    output: schema.object({}),
}).handle(async (call) => {
    // require a requested deletion
    const target = call.target as { readonly deletionRequestedAt: number | null };
    if (target.deletionRequestedAt === null) {
        throw new ServiceError("CONFLICT", { message: `${call.object.name} is not being deleted` });
    }

    // delete at the loaded revision
    await call.remove();

    return {};
});

/** The system methods a controlled object's controller calls. */
export type ControlledMethodMap<Controlled> = Controlled extends true
    ? { readonly observe: typeof observe; readonly finalize: typeof finalize }
    : {};

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
    /** Record a controller's observation and keep the last transition time while the status holds. */
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
    methods: () => ({ observe, finalize }),
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
