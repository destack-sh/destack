import { schema } from "@destack/schema";

/** A controller's observation of a record. */
export const Condition = schema.object({
    /** Whether the condition holds, or has not been established. */
    status: schema.enum(["true", "false", "unknown"]),
    /** The desired generation evaluated by the controller. */
    observedGeneration: schema.number().int().min(1),
    /** The machine-readable explanation. */
    reason: schema.string().min(1),
    /** The human-readable explanation. */
    message: schema.string(),
    /** The last change of condition status, in UTC epoch milliseconds. */
    lastTransitionAt: schema.number().int().min(0),
});

/** A controller's observation of a record. */
export type Condition = schema.Infer<typeof Condition>;

/** Conditions keyed by their unique domain-specific names. */
export const Conditions = schema.record(schema.string().min(1), Condition);
