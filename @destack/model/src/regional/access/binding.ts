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
import { space } from "../space/space.ts";

import { role } from "./role.ts";
import { serviceAccount } from "./service.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

/** Grant a space role to a global subject or a local workload identity. */
export const roleBinding = table(
    "role_binding",
    {
        ...recordColumns("role-binding"),
        ...provenanceColumns(),
        /** The account administering this space and its global subjects. */
        accountId: identifier("account_id", "account").notNull(),
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
            columns: [binding.accountId, binding.spaceId],
            foreignColumns: [space.accountId, space.id],
        }),
        foreignKey({
            columns: [binding.spaceId, binding.serviceAccountId],
            foreignColumns: [serviceAccount.spaceId, serviceAccount.id],
        }),
        check(
            "role_binding_subject",
            sql`CAST(${binding.accountMembershipId} IS NOT NULL AS integer) + CAST(${binding.groupId} IS NOT NULL AS integer) + CAST(${binding.serviceAccountId} IS NOT NULL AS integer) + CAST(${binding.accountServiceAccountId} IS NOT NULL AS integer) = 1`,
        ),
        check(
            "role_binding_expiry",
            sql`${binding.expiresAt} IS NULL OR ${binding.expiresAt} > ${binding.createdAt}`,
        ),
        uniqueIndex("role_binding_active")
            .on(
                binding.roleId,
                binding.spaceId,
                sql`coalesce(${binding.accountMembershipId}, ${binding.groupId}, ${binding.serviceAccountId}, ${binding.accountServiceAccountId})`,
            )
            .where(sql`${binding.revokedAt} IS NULL`),
    ],
);

/** A scoped role grant. */
export type RoleBinding = Select<typeof roleBinding>;
