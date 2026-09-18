import { defineRelationsPart } from "@destack/db";
import { account } from "../account/account.ts";
import { signInRequest } from "../account/signin.ts";
import { roleBinding } from "../account/binding.ts";
import { connectedAccount } from "../account/connection.ts";
import { group, groupMembership } from "../account/group.ts";
import { identity } from "../account/identity.ts";
import { accountInvitation } from "../account/invitation.ts";
import { accountMembership } from "../account/membership.ts";
import { permission } from "../account/permission.ts";
import { preference } from "../account/preference.ts";
import { role } from "../account/role.ts";
import { session } from "../account/session.ts";
import { serviceAccount, serviceToken } from "../account/software.ts";
import { user } from "../account/user.ts";
import { device } from "../host/device.ts";
import { installation } from "../space/installation.ts";
import { space } from "../space/space.ts";

/** Query relationships for account records. */
export const accountRelations = defineRelationsPart({
    user,
    account,
    space,
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
    permission,
    preference,
    device,
    session,
    installation,
    serviceToken,
}, (relation) => ({
    account: {
        user: relation.one.user({
            from: [relation.account.userId],
            to: [relation.user.id],
            optional: true,
        }),
        spaces: relation.many.space({ from: relation.account.id, to: relation.space.accountId }),
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
        space: relation.one.space({
            from: [relation.roleBinding.accountId, relation.roleBinding.spaceId],
            to: [relation.space.accountId, relation.space.id],
            optional: true,
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
    permission: {
        role: relation.one.role({
            from: [relation.permission.roleId],
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
        space: relation.one.space({
            from: [relation.serviceAccount.accountId, relation.serviceAccount.spaceId],
            to: [relation.space.accountId, relation.space.id],
            optional: true,
        }),
        installation: relation.one.installation({
            from: [relation.serviceAccount.spaceId, relation.serviceAccount.installationId],
            to: [relation.installation.spaceId, relation.installation.id],
            optional: true,
        }),
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
