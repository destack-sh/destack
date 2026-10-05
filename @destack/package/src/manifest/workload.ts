import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, Entrypoint } from "../definition/package.ts";
import { DeclarationReference } from "../declare/declaration.ts";
import { ComputeDefinition } from "../definition/compute.ts";
import { Capabilities, CapabilityName } from "../definition/capability.ts";

/** A tier a universe places workloads in, one instance per unit: the universe, each residency, each space or each host. */
export const Tier = defineSchema(schema.enum(["universe", "residency", "space", "host"]));
/** A tier a universe places workloads in. */
export type Tier = schema.Infer<typeof Tier>;

/** The placements a universe's deployer chooses from for a platform workload, as its deployment shape supports them. */
const Placement = schema.array(Tier).min(1);

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
            /** The placements a universe chooses from for the workload, absent for a workload spaces install. */
            placement: Placement.exactOptional(),
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
        /** What the workload may reach beyond its sandbox, as its package declares it. */
        capabilities: Capabilities,
        /** The placements a universe chooses from for the workload, absent for a workload spaces install. */
        placement: Placement.exactOptional(),
    }),
);
/** A workload located in a compiled output with the declarations its code reaches. */
export type WorkloadDescription = schema.Infer<typeof WorkloadDescription>;
