import { defineSchema, Instant, schema } from "@destack/schema";

/** A controller's observation of a record. */
export const StatusCondition = defineSchema(
    schema.object({
        /** Whether the condition is true, or has not been established. */
        status: schema.enum(["true", "false", "unknown"]),
        /** The desired generation the controller evaluated. */
        observedGeneration: schema.number().int().min(1),
        /** The machine-readable explanation. */
        reason: schema.string().min(1),
        /** The human-readable explanation. */
        message: schema.string(),
        /** The last change of condition status, in UTC epoch milliseconds. */
        lastTransitionAt: Instant,
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
        /** The conditions by name, their transition times kept while their status stays. */
        conditions: schema.record(
            schema.string().min(1),
            StatusCondition.omit({ observedGeneration: true, lastTransitionAt: true }),
        ),
        /** The observed fields the controller writes. */
        fields: schema.record(schema.string(), schema.json()).exactOptional(),
    }),
);
/** A controller's report on one record. */
export type Observation = schema.Infer<typeof Observation>;

/** Record a controller's observation of a condition, keeping its last transition time while the status stays. */
export function observeCondition(
    previous: StatusCondition | undefined,
    observation: Omit<StatusCondition, "lastTransitionAt">,
    now: number,
): StatusCondition {
    return {
        ...observation,
        lastTransitionAt: previous?.status === observation.status ? previous.lastTransitionAt : now,
    };
}
