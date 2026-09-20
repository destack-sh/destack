import { defineSchema, identifier, schema } from "@destack/schema";
import { Digest } from "@destack/package/file";
import { NetworkPolicyDefinition, PackagePolicyDefinition } from "@destack/space";

/** Exact policy revisions selected for a deployment's installation and workload. */
export const DeploymentPolicies = defineSchema(schema.object({
    /** Each package policy must permit the selected package graph. */
    packages: schema.array(schema.object({
        /** The region administering this revision. */
        regionId: identifier("region"),
        /** The immutable revision, resolved through its policy authority. */
        revisionId: identifier("package-policy-revision"),
        /** The canonical definition digest. */
        digest: Digest,
        /** The captured definition used to evaluate this deployment. */
        definition: PackagePolicyDefinition,
    })),
    /** Each network policy must permit the connection; hosts enforce their intersection. */
    network: schema.array(schema.object({
        /** The region administering this revision. */
        regionId: identifier("region"),
        /** The immutable revision, resolved through its policy authority. */
        revisionId: identifier("network-policy-revision"),
        /** The canonical definition digest. */
        digest: Digest,
        /** The captured definition supplied to the execution host. */
        definition: NetworkPolicyDefinition,
    })),
}));
/** Policy definitions retained with a deployment. */
export type DeploymentPolicies = schema.Infer<typeof DeploymentPolicies>;
