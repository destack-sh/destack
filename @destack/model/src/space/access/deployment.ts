import { defineSchema, identifier, schema } from "@destack/schema";
import { Digest } from "@destack/package/file";
import { NetworkPolicyDefinition, PackagePolicyDefinition } from "@destack/space";

/** The account or space administering a policy, independent of its current location. */
export const PolicyOwner = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** An account-wide policy. */
            kind: schema.literal("account"),
            /** The account whose regional policy service retains the revision. */
            accountId: identifier("account"),
        }),
        schema.object({
            /** A space, installation or workload policy. */
            kind: schema.literal("space"),
            /** The space whose administrator retains the revision. */
            spaceId: identifier("space"),
        }),
    ]),
);
/** The owner through which policy revisions are resolved. */
export type PolicyOwner = schema.Infer<typeof PolicyOwner>;

/** Exact policy revisions selected for a deployment's installation and workload. */
export const DeploymentPolicies = defineSchema(
    schema.object({
        /** Each package policy must permit the selected package graph. */
        packages: schema.array(
            schema.object({
                /** The owner through which the revision is resolved. */
                owner: PolicyOwner,
                /** The immutable revision, resolved through its policy authority. */
                revisionId: identifier("package-policy-revision"),
                /** The canonical definition digest. */
                digest: Digest,
                /** The captured definition used to evaluate this deployment. */
                definition: PackagePolicyDefinition,
            }),
        ),
        /** Each network policy must permit the connection; hosts enforce their intersection. */
        network: schema.array(
            schema.object({
                /** The owner through which the revision is resolved. */
                owner: PolicyOwner,
                /** The immutable revision, resolved through its policy authority. */
                revisionId: identifier("network-policy-revision"),
                /** The canonical definition digest. */
                digest: Digest,
                /** The captured definition supplied to the execution host. */
                definition: NetworkPolicyDefinition,
            }),
        ),
    }),
);
/** Policy definitions retained with a deployment. */
export type DeploymentPolicies = schema.Infer<typeof DeploymentPolicies>;
