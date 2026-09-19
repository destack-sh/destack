import { defineSchema, schema } from "@destack/schema";
import { PackageError } from "../error/index.ts";

/** CPU and memory capacity assigned to one running instance. */
export const ComputeResources = defineSchema(schema.object({
    /** CPU capacity in cores. */
    cpu: schema.number().positive().optional(),
    /** Memory capacity in MiB. */
    memory: schema.number().int().positive().optional(),
}));

/** CPU and memory capacity assigned to one running instance. */
export type ComputeResources = schema.Infer<typeof ComputeResources>;

/** Capacity and lifecycle policy for a workload. */
export const ComputeDefinition = defineSchema(
    schema.object({
        /** Minimum capacity requested when scheduling an instance. */
        requests: ComputeResources.optional(),
        /** Maximum capacity allowed for an instance. */
        limits: ComputeResources.optional(),
        /** Scaling bounds, including whether idle execution may stop. */
        scaling: schema.object({
            /** Minimum warm instances; zero permits stopping all idle instances. */
            minInstances: schema.number().int().nonnegative().optional(),
            /** Maximum simultaneous instances. */
            maxInstances: schema.number().int().positive().optional(),
        }).optional(),
        /** Time in milliseconds to retain an idle instance. */
        idleTimeout: schema.number().int().nonnegative().optional(),
        /** Time in milliseconds allowed for graceful shutdown. */
        shutdownTimeout: schema.number().int().positive().optional(),
        /** CPU time allowed per invocation in milliseconds. */
        cpuTime: schema.number().int().positive().optional(),
    }),
);

/** Capacity and lifecycle policy for a workload. */
export type ComputeDefinition = schema.Infer<typeof ComputeDefinition>;

/** Merge workload capacity settings and reject contradictory bounds. */
export function mergeCompute(
    defaults: ComputeDefinition = {},
    override: ComputeDefinition = {},
): ComputeDefinition {
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
        throw new PackageError("INVALID_DEFINITION", "Minimum scaling exceeds maximum scaling.");
    }

    // compare each requested resource with its corresponding limit
    for (const resource of ["cpu", "memory"] as const) {
        const request = compute.requests?.[resource];
        const limit = compute.limits?.[resource];
        if (request !== undefined && limit !== undefined && request > limit) {
            throw new PackageError("INVALID_DEFINITION", `${resource} request exceeds its limit.`);
        }
    }

    return compute;
}
