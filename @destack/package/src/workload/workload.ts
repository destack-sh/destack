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
            compute: ComputeDefinition.optional(),
        })
        .strict(),
);
/** A unit of deployment, as its declaration defines it. */
export type WorkloadDefinition = schema.Infer<typeof WorkloadDefinition>;

/** A workload located in a compiled output with the declarations its code reaches. */
export const WorkloadDescription = defineSchema(
    schema.object({
        /** The package export exposing the workload declaration. */
        entrypoint: Entrypoint,
        /** The export name of the workload declaration within the entrypoint. */
        export: schema.string().min(1),
        /** Service declarations of this package reachable from the workload. */
        services: schema.array(DeclarationReference),
        /** Schedule declarations of this package reachable from the workload. */
        schedules: schema.array(DeclarationReference),
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
