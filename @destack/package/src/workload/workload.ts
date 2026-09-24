import { defineSchema, schema } from "@destack/schema";
import { DeclarationName } from "../definition/package.ts";
import { DeclarationReference } from "../declare/declaration.ts";
import { ComputeDefinition } from "../definition/compute.ts";
import { Entrypoint } from "../definition/package.ts";

/** Code deployed and scaled together. */
export const WorkloadDefinition = defineSchema(
    schema
        .object({
            /** The exported module providing start(context) and declared schedule callbacks. */
            entrypoint: Entrypoint,
            /** Named services collected from code. */
            services: schema.array(DeclarationName).optional(),
            /** Named schedules delivered by the host scheduler. */
            schedules: schema.array(DeclarationName).optional(),
            /** Workload overrides of package compute defaults. */
            compute: ComputeDefinition.optional(),
        })
        .strict(),
);
/** Code deployed and scaled together. */
export type WorkloadDefinition = schema.Infer<typeof WorkloadDefinition>;

/** A workload checked against its declarations and generated output. */
export const WorkloadDescription = defineSchema(
    WorkloadDefinition.extend({
        /** Resource declarations collected from the workload's module dependencies. */
        resources: schema.array(DeclarationReference),
        /** Secret declarations collected from the workload's module dependencies. */
        secrets: schema.array(DeclarationReference),
        /** Service connection declarations collected from the workload's module dependencies. */
        connections: schema.array(DeclarationReference),
        /** The effective compute settings. */
        compute: ComputeDefinition,
    }),
);
/** A workload checked against its declarations and generated output. */
export type WorkloadDescription = schema.Infer<typeof WorkloadDescription>;
