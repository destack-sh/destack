import { DeclarationReference } from "@destack/package/declare";
import { DeclarationName } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { TriggerDescription } from "../trigger/trigger.ts";

/** A workflow in the manifest: its schemas, waits, retries, timeout, expiry and limits. */
export const WorkflowDescription = defineSchema(
    schema.object({
        /** The package-local workflow name. */
        name: DeclarationName,
        /** The JSON Schema of a run's input. */
        input: schema.jsonObject(),
        /** The JSON Schema of a run's output, absent for a workflow returning nothing. */
        output: schema.jsonObject().exactOptional(),
        /** The object types whose writes wake waiting runs. */
        waits: schema.array(DeclarationReference),
        /** How a run retries a failed advance. */
        retries: schema.object({
            /** The wait before the second attempt, in milliseconds. */
            initialInterval: schema.number().nonnegative(),
            /** The growth factor of each wait. */
            backoffCoefficient: schema.number().positive(),
            /** The longest wait between attempts, in milliseconds. */
            maximumInterval: schema.number().nonnegative(),
            /** The most attempts, absent to retry until the timeout. */
            maximumAttempts: schema.number().int().positive().exactOptional(),
            /** Whether waits spread uniformly below their interval. */
            jitter: schema.literal("full").exactOptional(),
        }),
        /** How long a run lasts at most, in milliseconds. */
        timeout: schema.number().int().positive().exactOptional(),
        /** How long an ended run stays, in milliseconds. */
        expiring: schema.number().int().positive().exactOptional(),
        /** How often runs start, how many advance at once, and whose limit they count against. */
        limits: schema.object({
            /** The runs started per period. */
            rate: schema
                .object({
                    /** The runs per period. */
                    count: schema.number().int().positive(),
                    /** The period, in milliseconds. */
                    every: schema.number().int().positive(),
                    /** The runs started at once after idling. */
                    burst: schema.number().int().positive(),
                })
                .exactOptional(),
            /** The runs advancing at once. */
            concurrency: schema.object({ slots: schema.number().int().positive() }).exactOptional(),
            /** Whose limit a run counts against: its scope's, its actor's, or an input value's. */
            key: schema.union([
                schema.enum(["scope", "caller"]),
                schema.object({ input: schema.string().min(1) }),
            ]),
        }),
        /** Whether a run holds its concurrency slot from its start to its end. */
        isSingleton: schema.boolean(),
        /** The declared triggers starting runs. */
        triggers: schema.array(TriggerDescription),
    }),
);
/** A workflow in the manifest. */
export type WorkflowDescription = schema.Infer<typeof WorkflowDescription>;
