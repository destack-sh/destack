import { defineSchema, schema } from "@destack/schema";
import { DeclarationName } from "@destack/package";
import { PackagePolicyDefinition } from "./package.ts";
import { NetworkPolicyDefinition } from "./network.ts";

/** Stack-managed policies applied throughout a space. */
export const SpacePolicy = defineSchema(
    schema.object({
        /** Package admission rules. */
        packages: PackagePolicyDefinition.optional(),
        /** Outbound access intersected with account and host restrictions. */
        network: NetworkPolicyDefinition.optional(),
    }),
);
/** The policies declared by a space configuration. */
export type SpacePolicy = schema.Infer<typeof SpacePolicy>;

/** Network restrictions applied to an installation and its named workloads. */
export const InstallationPolicy = defineSchema(
    schema.object({
        /** Restrictions shared by every workload in the installation. */
        network: NetworkPolicyDefinition.optional(),
        /** Additional restrictions for individual package workloads. */
        workloads: schema
            .record(
                DeclarationName,
                schema.object({
                    /** Outbound access intersected with installation, space, account, and host restrictions. */
                    network: NetworkPolicyDefinition,
                }),
            )
            .optional(),
    }),
);
/** Installation and workload policy declarations. */
export type InstallationPolicy = schema.Infer<typeof InstallationPolicy>;
