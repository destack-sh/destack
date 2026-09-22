import { defineRelationsPart } from "@destack/db";
import { account } from "./account.ts";
import { accountSource, accountRevision } from "./source.ts";
import { environment } from "./environment.ts";
import { organisation, organisationMembership, organisationInvitation } from "./organisation.ts";
import { roleBinding } from "./binding.ts";
import { connectedAccount } from "./connection.ts";
import { group, groupMembership } from "./group.ts";
import { accountInvitation } from "./invitation.ts";
import { accountMembership } from "./membership.ts";
import { rolePermission } from "./permission.ts";
import { preference } from "./preference.ts";
import { role } from "./role.ts";
import { serviceAccount, serviceToken } from "./service.ts";
import { user } from "./user.ts";
import { device } from "../host/device.ts";
import { region } from "../host/region.ts";
import { repository } from "../package/repository.ts";

/** Query relationships for account records. */
export const accountRelations = defineRelationsPart(
    {
        user,
        account,
        environment,
        accountSource,
        accountRevision,
        organisation,
        organisationMembership,
        organisationInvitation,
        role,
        roleBinding,
        accountMembership,
        group,
        serviceAccount,
        connectedAccount,
        groupMembership,
        accountInvitation,
        rolePermission,
        preference,
        device,
        serviceToken,
        region,
        repository,
    },
    (relation) => ({
        environment: {
            account: relation.one.account({
                from: relation.environment.accountId,
                to: relation.account.id,
                optional: false,
            }),
        },
        accountSource: {
            repository: relation.one.repository({
                from: [relation.accountSource.accountId, relation.accountSource.repositoryId],
                to: [relation.repository.accountId, relation.repository.id],
                optional: false,
            }),
            account: relation.one.account({
                from: relation.accountSource.accountId,
                to: relation.account.id,
                optional: false,
            }),
            applied: relation.one.accountRevision({
                from: [relation.accountSource.accountId, relation.accountSource.appliedRevisionId],
                to: [relation.accountRevision.accountId, relation.accountRevision.id],
                optional: true,
            }),
        },
        accountRevision: {
            account: relation.one.account({
                from: relation.accountRevision.accountId,
                to: relation.account.id,
                optional: false,
            }),
        },
        user: {
            accounts: relation.many.account({
                from: relation.user.id,
                to: relation.account.userId,
            }),
            memberships: relation.many.organisationMembership({
                from: relation.user.id,
                to: relation.organisationMembership.userId,
            }),
        },
        organisation: {
            accounts: relation.many.account({
                from: relation.organisation.id,
                to: relation.account.organisationId,
            }),
            memberships: relation.many.organisationMembership({
                from: relation.organisation.id,
                to: relation.organisationMembership.organisationId,
            }),
        },
        organisationMembership: {
            organisation: relation.one.organisation({
                from: relation.organisationMembership.organisationId,
                to: relation.organisation.id,
                optional: false,
            }),
            user: relation.one.user({
                from: relation.organisationMembership.userId,
                to: relation.user.id,
                optional: false,
            }),
        },
        organisationInvitation: {
            organisation: relation.one.organisation({
                from: relation.organisationInvitation.organisationId,
                to: relation.organisation.id,
                optional: false,
            }),
            inviter: relation.one.user({
                from: relation.organisationInvitation.invitedBy,
                to: relation.user.id,
                optional: false,
            }),
            accepter: relation.one.user({
                from: relation.organisationInvitation.acceptedBy,
                to: relation.user.id,
                optional: true,
            }),
        },
        account: {
            source: relation.one.accountSource({
                from: relation.account.id,
                to: relation.accountSource.accountId,
                optional: true,
            }),
            organisation: relation.one.organisation({
                from: relation.account.organisationId,
                to: relation.organisation.id,
                optional: true,
            }),
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
    }),
);
