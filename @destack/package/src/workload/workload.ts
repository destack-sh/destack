import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, Entrypoint } from "../definition/package.ts";
import { DeclarationReference } from "../declare/declaration.ts";
import { ComputeDefinition } from "../definition/compute.ts";

/** A unit of deployment, as its declaration defines it. */
export const WorkloadDefinition = defineSchema(
    schema
        .object({
            /** The package-local workload name. */
            name: DeclarationName,
            /** Capacity and lifecycle policy for each instance. */
            compute: ComputeDefinition.exactOptional(),
        })
        .strict(),
);
/** A unit of deployment, as its declaration defines it. */
export type WorkloadDefinition = schema.Infer<typeof WorkloadDefinition>;

/** A workload located in a compiled output with the declarations its code reaches. */
export const WorkloadDescription = defineSchema(
    schema.object({
        /** The package entrypoint running the workload. */
        entrypoint: Entrypoint,
        /** Service declarations of this package reachable from the workload. */
        services: schema.array(DeclarationReference),
        /** Trigger declarations (schedules, webhooks, subscriptions) of this package reachable from the workload. */
        triggers: schema.array(DeclarationReference),
        /** Resource declarations reachable from the workload. */
        resources: schema.array(DeclarationReference),
        /** Secret declarations reachable from the workload. */
        secrets: schema.array(DeclarationReference),
        /** Service connection declarations reachable from the workload. */
        connections: schema.array(DeclarationReference),
        /** Capacity and lifecycle policy for each instance. */
        compute: ComputeDefinition,
    }),
);
/** A workload located in a compiled output with the declarations its code reaches. */
export type WorkloadDescription = schema.Infer<typeof WorkloadDescription>;
