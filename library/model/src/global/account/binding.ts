import {
    check,
    foreignKey,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    uniqueIndex,
} from "@destack/db";
import { spaceDirectory } from "../directory/space.ts";
import { group } from "./group.ts";
import { accountMembership } from "./membership.ts";
import { role } from "./role.ts";
import { serviceAccount } from "./service.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

/** Grant an account role to one member, group, or software identity. */
export const roleBinding = table("role_binding", {
    ...recordColumns("role-binding"),
    ...provenanceColumns(),
    /** The account containing the role, subject, and optional space. */
    accountId: identifier("account_id", "account").notNull(),
    /** The granted role. */
    roleId: identifier("role_id", "role").notNull(),
    /** Restrict the grant to one space; the role's own scope always also applies. */
    spaceId: identifier("space_id", "space"),
    /** The member receiving this grant. */
    accountMembershipId: identifier("account_membership_id", "account-membership"),
    /** The group receiving this grant. */
    groupId: identifier("group_id", "group"),
    /** The software identity receiving this grant. */
    serviceAccountId: identifier("service_account_id", "service-account"),
    /** Optional grant expiry. */
    expiresAt: integer("expires_at"),
    /** Explicit revocation time. */
    revokedAt: integer("revoked_at"),
}, (binding) => [
    foreignKey({
        columns: [binding.accountId, binding.spaceId],
        foreignColumns: [spaceDirectory.accountId, spaceDirectory.id],
    }).onDelete("restrict"),
    ...provenanceChecks("role_binding", binding),
    foreignKey({
        columns: [binding.accountId, binding.roleId],
        foreignColumns: [role.accountId, role.id],
    }),
    foreignKey({
        columns: [binding.accountId, binding.accountMembershipId],
        foreignColumns: [accountMembership.accountId, accountMembership.id],
    }),
    foreignKey({
        columns: [binding.accountId, binding.groupId],
        foreignColumns: [group.accountId, group.id],
    }),
    foreignKey({
        columns: [binding.accountId, binding.serviceAccountId],
        foreignColumns: [serviceAccount.accountId, serviceAccount.id],
    }),
    check(
        "role_binding_subject",
        sql`CAST(${binding.accountMembershipId} IS NOT NULL AS integer) + CAST(${binding.groupId} IS NOT NULL AS integer) + CAST(${binding.serviceAccountId} IS NOT NULL AS integer) = 1`,
    ),
    check(
        "role_binding_expiry",
        sql`${binding.expiresAt} IS NULL OR ${binding.expiresAt} > ${binding.createdAt}`,
    ),
    uniqueIndex("role_binding_active").on(
        binding.roleId,
        sql`coalesce(${binding.spaceId}, '')`,
        sql`coalesce(${binding.accountMembershipId}, ${binding.groupId}, ${binding.serviceAccountId})`,
    ).where(sql`${binding.revokedAt} IS NULL`),
]);

/** A scoped role grant. */
export type RoleBinding = Select<typeof roleBinding>;
