import { defineSchema, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";
import { PackagePolicyDefinition } from "./package.ts";
import { NetworkPolicyDefinition } from "./network.ts";

/** Source-managed policies applied throughout a space. */
export const SpacePolicies = defineSchema(
    schema.object({
        /** Package admission rules. */
        packages: PackagePolicyDefinition.optional(),
        /** Outbound access intersected with account and host restrictions. */
        network: NetworkPolicyDefinition.optional(),
    }),
);
/** The policies declared by a space configuration. */
export type SpacePolicies = schema.Infer<typeof SpacePolicies>;

/** Network restrictions applied to an installation and its named workloads. */
export const InstallationPolicies = defineSchema(
    schema.object({
        /** Restrictions shared by every workload in the installation. */
        network: NetworkPolicyDefinition.optional(),
        /** Additional restrictions for individual package workloads. */
        workloads: schema
            .record(
                ResourceName,
                schema.object({
                    /** Outbound access intersected with installation, space, account, and host restrictions. */
                    network: NetworkPolicyDefinition,
                }),
            )
            .optional(),
    }),
);
/** Installation and workload policy declarations. */
export type InstallationPolicies = schema.Infer<typeof InstallationPolicies>;
