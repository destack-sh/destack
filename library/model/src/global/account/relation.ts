import { defineRelationsPart } from "@destack/db";
import { account } from "./account.ts";
import { signInRequest } from "../authentication/signin.ts";
import { roleBinding } from "./binding.ts";
import { connectedAccount } from "./connection.ts";
import { group, groupMembership } from "./group.ts";
import { identity } from "../authentication/identity.ts";
import { accountInvitation } from "./invitation.ts";
import { accountMembership } from "./membership.ts";
import { rolePermission } from "./permission.ts";
import { preference } from "./preference.ts";
import { role } from "./role.ts";
import { session } from "../authentication/session.ts";
import { serviceAccount, serviceToken } from "./service.ts";
import { user } from "./user.ts";
import { device } from "../host/device.ts";
import { region } from "../host/region.ts";

/** Query relationships for account records. */
export const accountRelations = defineRelationsPart({
    user,
    account,
    signInRequest,
    role,
    roleBinding,
    accountMembership,
    group,
    serviceAccount,
    connectedAccount,
    groupMembership,
    identity,
    accountInvitation,
    rolePermission,
    preference,
    device,
    session,
    serviceToken,
    region,
}, (relation) => ({
    account: {
        networkPolicyRegion: relation.one.region({
            from: [relation.account.networkPolicyRegionId],
            to: [relation.region.id],
            optional: true,
        }),
        packagePolicyRegion: relation.one.region({
            from: [relation.account.packagePolicyRegionId],
            to: [relation.region.id],
            optional: true,
        }),
        user: relation.one.user({
            from: [relation.account.userId],
            to: [relation.user.id],
            optional: true,
        }),
    },
    signInRequest: {
        user: relation.one.user({
            from: [relation.signInRequest.userId],
            to: [relation.user.id],
            optional: true,
        }),
    },
    roleBinding: {
        role: relation.one.role({
            from: [relation.roleBinding.accountId, relation.roleBinding.roleId],
            to: [relation.role.accountId, relation.role.id],
            optional: false,
        }),
        accountMembership: relation.one.accountMembership({
            from: [relation.roleBinding.accountId, relation.roleBinding.accountMembershipId],
            to: [relation.accountMembership.accountId, relation.accountMembership.id],
            optional: true,
        }),
        group: relation.one.group({
            from: [relation.roleBinding.accountId, relation.roleBinding.groupId],
            to: [relation.group.accountId, relation.group.id],
            optional: true,
        }),
        serviceAccount: relation.one.serviceAccount({
            from: [relation.roleBinding.accountId, relation.roleBinding.serviceAccountId],
            to: [relation.serviceAccount.accountId, relation.serviceAccount.id],
            optional: true,
        }),
    },
    connectedAccount: {
        account: relation.one.account({
            from: [relation.connectedAccount.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        user: relation.one.user({
            from: [relation.connectedAccount.userId],
            to: [relation.user.id],
            optional: false,
        }),
    },
    group: {
        account: relation.one.account({
            from: [relation.group.accountId],
            to: [relation.account.id],
            optional: false,
        }),
    },
    groupMembership: {
        group: relation.one.group({
            from: [relation.groupMembership.accountId, relation.groupMembership.groupId],
            to: [relation.group.accountId, relation.group.id],
            optional: false,
        }),
        accountMembership: relation.one.accountMembership({
            from: [
                relation.groupMembership.accountId,
                relation.groupMembership.accountMembershipId,
            ],
            to: [relation.accountMembership.accountId, relation.accountMembership.id],
            optional: false,
        }),
    },
    identity: {
        user: relation.one.user({
            from: [relation.identity.userId],
            to: [relation.user.id],
            optional: false,
        }),
    },
    accountInvitation: {
        account: relation.one.account({
            from: [relation.accountInvitation.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        inviter: relation.one.user({
            from: [relation.accountInvitation.invitedBy],
            to: [relation.user.id],
            optional: false,
        }),
        accepter: relation.one.user({
            from: [relation.accountInvitation.acceptedBy],
            to: [relation.user.id],
            optional: true,
        }),
    },
    accountMembership: {
        account: relation.one.account({
            from: [relation.accountMembership.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        user: relation.one.user({
            from: [relation.accountMembership.userId],
            to: [relation.user.id],
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
    preference: {
        account: relation.one.account({
            from: [relation.preference.accountId],
            to: [relation.account.id],
            optional: false,
        }),
        user: relation.one.user({
            from: [relation.preference.userId],
            to: [relation.user.id],
            optional: true,
        }),
        device: relation.one.device({
            from: [relation.preference.deviceId],
            to: [relation.device.id],
            optional: true,
        }),
    },
    role: {
        account: relation.one.account({
            from: [relation.role.accountId],
            to: [relation.account.id],
            optional: false,
        }),
    },
    session: {
        user: relation.one.user({
            from: [relation.session.userId],
            to: [relation.user.id],
            optional: false,
        }),
        device: relation.one.device({
            from: [relation.session.deviceId],
            to: [relation.device.id],
            optional: true,
        }),
    },
    serviceAccount: {
        account: relation.one.account({
            from: [relation.serviceAccount.accountId],
            to: [relation.account.id],
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
