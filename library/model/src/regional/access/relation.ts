import { defineRelationsPart } from "@destack/db";
import { role } from "./role.ts";
import { rolePermission } from "./permission.ts";
import { roleBinding } from "./binding.ts";
import { serviceAccount, serviceToken } from "./service.ts";
import { space } from "../space/space.ts";
import { installation } from "../space/installation.ts";

/** SQL relationships within a space's administrative records. */
export const accessRelations = defineRelationsPart({
    role,
    rolePermission,
    roleBinding,
    serviceAccount,
    serviceToken,
    space,
    installation,
}, (relation) => ({
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
}));
