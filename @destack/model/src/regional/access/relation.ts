import { defineRelationsPart } from "@destack/db";
import { role } from "./role.ts";
import { rolePermission } from "./permission.ts";
import { roleBinding } from "./binding.ts";
import { serviceAccount, serviceToken } from "./service.ts";
import { space } from "../space/space.ts";
import { installation } from "../space/installation.ts";
import { networkPolicy, networkPolicyRevision } from "./network.ts";
import { packagePolicy, packagePolicyRevision } from "./package.ts";

/** SQL relationships within a space's administrative records. */
export const accessRelations = defineRelationsPart(
    {
        role,
        rolePermission,
        roleBinding,
        serviceAccount,
        serviceToken,
        space,
        installation,
        networkPolicy,
        networkPolicyRevision,
        packagePolicy,
        packagePolicyRevision,
    },
    (relation) => ({
        packagePolicy: {
            space: relation.one.space({
                from: relation.packagePolicy.spaceId,
                to: relation.space.id,
                optional: true,
            }),
            currentRevision: relation.one.packagePolicyRevision({
                from: [relation.packagePolicy.id, relation.packagePolicy.currentRevisionId],
                to: [relation.packagePolicyRevision.policyId, relation.packagePolicyRevision.id],
                optional: true,
            }),
        },
        packagePolicyRevision: {
            policy: relation.one.packagePolicy({
                from: relation.packagePolicyRevision.policyId,
                to: relation.packagePolicy.id,
                optional: false,
            }),
        },
        networkPolicy: {
            space: relation.one.space({
                from: [relation.networkPolicy.spaceId],
                to: [relation.space.id],
                optional: true,
            }),
            installation: relation.one.installation({
                from: [relation.networkPolicy.spaceId, relation.networkPolicy.installationId],
                to: [relation.installation.spaceId, relation.installation.id],
                optional: true,
            }),
            currentRevision: relation.one.networkPolicyRevision({
                from: [relation.networkPolicy.id, relation.networkPolicy.currentRevisionId],
                to: [relation.networkPolicyRevision.policyId, relation.networkPolicyRevision.id],
                optional: true,
            }),
        },
        networkPolicyRevision: {
            policy: relation.one.networkPolicy({
                from: [relation.networkPolicyRevision.policyId],
                to: [relation.networkPolicy.id],
                optional: false,
            }),
        },
        role: {
            space: relation.one.space({
                from: [relation.role.spaceId],
                to: [relation.space.id],
                optional: false,
            }),
        },
        rolePermission: {
            role: relation.one.role({
                from: [relation.rolePermission.roleId],
                to: [relation.role.id],
                optional: false,
            }),
        },
        roleBinding: {
            role: relation.one.role({
                from: [relation.roleBinding.spaceId, relation.roleBinding.roleId],
                to: [relation.role.spaceId, relation.role.id],
                optional: false,
            }),
            serviceAccount: relation.one.serviceAccount({
                from: [relation.roleBinding.spaceId, relation.roleBinding.serviceAccountId],
                to: [relation.serviceAccount.spaceId, relation.serviceAccount.id],
                optional: true,
            }),
        },
        serviceAccount: {
            installation: relation.one.installation({
                from: [relation.serviceAccount.spaceId, relation.serviceAccount.installationId],
                to: [relation.installation.spaceId, relation.installation.id],
                optional: false,
            }),
        },
        serviceToken: {
            serviceAccount: relation.one.serviceAccount({
                from: [relation.serviceToken.serviceAccountId],
                to: [relation.serviceAccount.id],
                optional: false,
            }),
        },
    }),
);
