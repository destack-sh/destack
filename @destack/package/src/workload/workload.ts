import { defineSchema, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";
import { ComputeDefinition } from "../package/compute.ts";
import { PackageId } from "../package/package.ts";

/** A declaration qualified by its source package. */
export const DeclarationReference = defineSchema(
    schema.object({
        /** The immutable identity of the declaring package. */
        packageId: PackageId,
        /** The name assigned by the domain declaration. */
        name: ResourceName,
    }),
);
/** A declaration qualified by its source package. */
export type DeclarationReference = schema.Infer<typeof DeclarationReference>;

/** A package export containing runnable code. */
export const Entrypoint = defineSchema(schema.string().regex(/^\.(?:\/[^\s*]+)?$(?![\s\S])/));

/** Code deployed and scaled together. */
export const WorkloadDefinition = defineSchema(
    schema
        .object({
            /** The exported module containing the workload handlers. */
            entrypoint: Entrypoint,
            /** Named services collected from code. */
            services: schema.array(ResourceName).optional(),
            /** Named schedules delivered by the host scheduler. */
            schedules: schema.array(ResourceName).optional(),
            /** Instance startup and shutdown exports. */
            lifecycle: schema
                .object({
                    /** Start background activity and resolve when ready. */
                    start: schema.string().min(1).optional(),
                    /** Drain background activity before stopping. */
                    stop: schema.string().min(1).optional(),
                })
                .optional(),
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
        /** The effective compute settings. */
        compute: ComputeDefinition,
    }),
);
/** A workload checked against its declarations and generated output. */
export type WorkloadDescription = schema.Infer<typeof WorkloadDescription>;
