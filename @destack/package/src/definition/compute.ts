import { defineSchema, schema } from "@destack/schema";
import { PackageError } from "../error/index.ts";

/** CPU and memory capacity assigned to one running instance. */
export const ComputeCapacity = defineSchema(
    schema.object({
        /** CPU capacity in cores. */
        cpu: schema.number().positive().exactOptional(),
        /** Memory capacity in MiB. */
        memory: schema.number().int().positive().exactOptional(),
    }),
);

/** CPU and memory capacity assigned to one running instance. */
export type ComputeCapacity = schema.Infer<typeof ComputeCapacity>;

/** The schema of a workload's capacity and lifecycle policy. */
const computeDefinitionSchema = defineSchema(
    schema.object({
        /** Minimum capacity requested when scheduling an instance. */
        requests: ComputeCapacity.exactOptional(),
        /** Maximum capacity allowed for an instance. */
        limits: ComputeCapacity.exactOptional(),
        /** Instance scaling bounds. */
        scaling: schema
            .object({
                /** Minimum warm instances, zero to stop every idle instance. */
                minInstances: schema.number().int().nonnegative().exactOptional(),
                /** Maximum simultaneous instances. */
                maxInstances: schema.number().int().positive().exactOptional(),
            })
            .exactOptional(),
        /** Time in milliseconds to retain an idle instance. */
        idleTimeout: schema.number().int().nonnegative().exactOptional(),
        /** Time in milliseconds allowed for graceful shutdown. */
        shutdownTimeout: schema.number().int().positive().exactOptional(),
        /** CPU time allowed per invocation in milliseconds. */
        cpuTime: schema.number().int().positive().exactOptional(),
    }),
);

/** Capacity and lifecycle policy for a workload. */
export type ComputeDefinition = schema.Infer<typeof computeDefinitionSchema>;

/** Capacity and lifecycle policy for a workload. */
export const ComputeDefinition = Object.assign(computeDefinitionSchema, { merge });

/** Merge workload capacity settings and reject contradictory bounds. */
function merge(
    defaults: ComputeDefinition = {},
    override: ComputeDefinition = {},
): ComputeDefinition {
    // merge overrides over defaults field by field
    const compute: ComputeDefinition = {
        ...defaults,
        ...override,
        requests: { ...defaults.requests, ...override.requests },
        limits: { ...defaults.limits, ...override.limits },
        scaling: { ...defaults.scaling, ...override.scaling },
    };

    // require the scaling interval to contain at least one valid instance count
    if (
        compute.scaling?.minInstances !== undefined &&
        compute.scaling?.maxInstances !== undefined &&
        compute.scaling.minInstances > compute.scaling.maxInstances
    ) {
        throw new PackageError("INVALID_DEFINITION", "minimum scaling exceeds maximum scaling");
    }

    // compare each requested resource with its corresponding limit
    for (const resource of ["cpu", "memory"] as const) {
        const request = compute.requests?.[resource];
        const limit = compute.limits?.[resource];
        if (request !== undefined && limit !== undefined && request > limit) {
            throw new PackageError("INVALID_DEFINITION", `${resource} request exceeds its limit`);
        }
    }

    return compute;
}
