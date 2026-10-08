import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, Entrypoint } from "../definition/package.ts";
import { DeclarationReference } from "../declare/declaration.ts";
import { ComputeDefinition } from "../definition/compute.ts";
import { Capabilities, CapabilityName } from "../definition/capability.ts";

/** A unit of deployment, as its declaration defines it. */
export const WorkloadDefinition = defineSchema(
    schema
        .object({
            /** The package-local workload name. */
            name: DeclarationName,
            /** Capacity and lifecycle policy for each instance. */
            compute: ComputeDefinition.exactOptional(),
            /** The package's capabilities the workload uses, every one when omitted. */
            capabilities: schema.array(CapabilityName).exactOptional(),
            /** The permissions the workload requests by scope, as `@destack/access` reads a `PermissionRequest`. */
            permissions: schema.record(schema.string(), schema.json()).exactOptional(),
        })
        .strict(),
);
/** A unit of deployment, as its declaration defines it. */
export type WorkloadDefinition = schema.Infer<typeof WorkloadDefinition>;

/** A workload located in a compiled output with the declarations its code imports. */
export const WorkloadDescription = defineSchema(
    schema.object({
        /** The package entrypoint running the workload. */
        entrypoint: Entrypoint,
        /** Service declarations of this package the workload imports. */
        services: schema.array(DeclarationReference),
        /** Trigger declarations (schedules, webhooks, subscriptions) of this package the workload imports. */
        triggers: schema.array(DeclarationReference),
        /** Resource declarations the workload imports. */
        resources: schema.array(DeclarationReference),
        /** Secret declarations the workload imports. */
        secrets: schema.array(DeclarationReference),
        /** Service connection declarations the workload imports. */
        connections: schema.array(DeclarationReference),
        /** Capacity and lifecycle policy for each instance. */
        compute: ComputeDefinition,
        /** What the workload may call beyond its sandbox, as its package declares it. */
        capabilities: Capabilities,
        /** The permissions the workload requests by scope, as `@destack/access` reads a `PermissionRequest`. */
        permissions: schema.record(schema.string(), schema.json()),
    }),
);
/** A workload located in a compiled output with the declarations its code imports. */
export type WorkloadDescription = schema.Infer<typeof WorkloadDescription>;
