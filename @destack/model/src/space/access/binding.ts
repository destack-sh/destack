import {
    check,
    foreignKey,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    uniqueIndex,
} from "@destack/db";
import { space } from "../space/space.ts";

import { role } from "./role.ts";
import { serviceAccount } from "./service.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

/** Grant a space role to a verified user, membership, group or workload. */
export const roleBinding = table(
    "role_binding",
    {
        ...recordColumns("role-binding"),
        ...provenanceColumns(),
        /** The authority of an account membership, group or account service identity. */
        accountId: identifier("account_id", "account"),
        /** The authority of a directly granted user identity. */
        userAuthority: text("user_authority"),
        /** The immutable user identifier assigned by that authority. */
        userId: text("user_id"),
        /** The granted role. */
        roleId: identifier("role_id", "role").notNull(),
        /** The space containing the granted role. */
        spaceId: identifier("space_id", "space").notNull(),
        /** The member receiving this grant. */
        accountMembershipId: identifier("account_membership_id", "account-membership"),
        /** The group receiving this grant. */
        groupId: identifier("group_id", "group"),
        /** The software identity receiving this grant. */
        serviceAccountId: identifier("service_account_id", "service-account"),
        /** An external software identity administered by the global account service. */
        accountServiceAccountId: identifier("account_service_account_id", "service-account"),
        /** Optional grant expiry. */
        expiresAt: integer("expires_at"),
        /** Explicit revocation time. */
        revokedAt: integer("revoked_at"),
    },
    (binding) => [
        ...provenanceChecks("role_binding", binding),
        foreignKey({
            columns: [binding.spaceId, binding.roleId],
            foreignColumns: [role.spaceId, role.id],
        }),
        foreignKey({
            columns: [binding.spaceId],
            foreignColumns: [space.id],
        }),
        foreignKey({
            columns: [binding.spaceId, binding.serviceAccountId],
            foreignColumns: [serviceAccount.spaceId, serviceAccount.id],
        }),
        check(
            "role_binding_subject",
            sql`CAST(${binding.accountMembershipId} IS NOT NULL AS integer) + CAST(${binding.groupId} IS NOT NULL AS integer) + CAST(${binding.serviceAccountId} IS NOT NULL AS integer) + CAST(${binding.accountServiceAccountId} IS NOT NULL AS integer) + CAST(${binding.userId} IS NOT NULL AS integer) = 1`,
        ),
        check(
            "role_binding_user",
            sql`(${binding.userAuthority} IS NULL) = (${binding.userId} IS NULL) AND (${binding.userId} IS NULL OR (length(${binding.userId}) > 0 AND length(${binding.userAuthority}) > 0))`,
        ),
        check(
            "role_binding_account",
            sql`(${binding.accountId} IS NOT NULL) = (${binding.accountMembershipId} IS NOT NULL OR ${binding.groupId} IS NOT NULL OR ${binding.accountServiceAccountId} IS NOT NULL)`,
        ),
        check(
            "role_binding_expiry",
            sql`${binding.expiresAt} IS NULL OR ${binding.expiresAt} > ${binding.createdAt}`,
        ),
        uniqueIndex("role_binding_active")
            .on(
                binding.roleId,
                binding.spaceId,
                sql`CASE WHEN ${binding.accountMembershipId} IS NOT NULL THEN 'membership' WHEN ${binding.groupId} IS NOT NULL THEN 'group' WHEN ${binding.serviceAccountId} IS NOT NULL THEN 'workload' WHEN ${binding.accountServiceAccountId} IS NOT NULL THEN 'service-account' ELSE 'user' END`,
                sql`coalesce(${binding.accountId}, ${binding.userAuthority}, '')`,
                sql`coalesce(${binding.accountMembershipId}, ${binding.groupId}, ${binding.serviceAccountId}, ${binding.accountServiceAccountId}, ${binding.userId})`,
            )
            .where(sql`${binding.revokedAt} IS NULL`),
    ],
);

/** A scoped role grant. */
export type RoleBinding = Select<typeof roleBinding>;
