import {
    check,
    dialectSQL,
    identifier,
    index,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { user } from "./user.ts";
import { organisation } from "./organisation.ts";
import { RESIDENCIES } from "../host/residency.ts";
import { region } from "../host/region.ts";

/** Account records. */
export const account = table(
    "account",
    {
        ...recordColumns("account"),
        /** The public account handle. */
        handle: text("handle").notNull().unique(),
        /** The displayed account name. */
        name: text("name").notNull(),
        /** The default jurisdiction copied into newly created spaces. */
        defaultResidency: text("default_residency", { enum: RESIDENCIES }).notNull(),
        /** The account-wide package policy, resolved in its authoritative region. */
        packagePolicyId: identifier("package_policy_id", "package-policy"),
        /** The regional database containing the account-wide policy. */
        packagePolicyRegionId: identifier("package_policy_region_id", "region").references(
            () => region.id,
            { onDelete: "restrict" },
        ),
        /** The account-wide network policy, resolved in its authoritative region. */
        networkPolicyId: identifier("network_policy_id", "network-policy"),
        /** The regional database containing the account-wide network policy. */
        networkPolicyRegionId: identifier("network_policy_region_id", "region").references(
            () => region.id,
            { onDelete: "restrict" },
        ),
        /** Suspension time; retained records remain available for recovery. */
        suspendedAt: integer("suspended_at"),
        /** Explicit deletion request, completed after retention and cleanup. */
        deletionRequestedAt: integer("deletion_requested_at"),
        /** Whether the account belongs to a person or organisation. */
        kind: text("kind", { enum: ["personal", "organisation"] }).notNull(),
        /** The user owning a personal account. */
        userId: identifier("user_id", "user").references(() => user.id, {
            onDelete: "restrict",
        }),
        /** The organisation owning an organisational account. */
        organisationId: identifier("organisation_id", "organisation").references(
            () => organisation.id,
            { onDelete: "restrict" },
        ),
    },
    (account) => [
        index("account_user").on(account.userId),
        index("account_organisation").on(account.organisationId),
        check("account_residency", sql`${account.defaultResidency} IN ('eu', 'us')`),
        check("account_kind", sql`${account.kind} IN ('personal', 'organisation')`),
        check(
            "account_network_policy",
            sql`
        (${account.networkPolicyId} IS NULL AND ${account.networkPolicyRegionId} IS NULL) OR
        (${account.networkPolicyId} IS NOT NULL AND ${account.networkPolicyRegionId} IS NOT NULL)
    `,
        ),
        check(
            "account_package_policy",
            sql`
        (${account.packagePolicyId} IS NULL AND ${account.packagePolicyRegionId} IS NULL) OR
        (${account.packagePolicyId} IS NOT NULL AND ${account.packagePolicyRegionId} IS NOT NULL)
    `,
        ),
        check(
            "account_user",
            sql`(${account.kind} = 'personal' AND ${account.userId} IS NOT NULL AND ${account.organisationId} IS NULL) OR (${account.kind} = 'organisation' AND ${account.userId} IS NULL AND ${account.organisationId} IS NOT NULL)`,
        ),
        check(
            "account_handle",
            dialectSQL({
                sqlite: sql`length(${account.handle}) BETWEEN 1 AND 63 AND ${account.handle} NOT GLOB '*[^a-z0-9-]*' AND ${account.handle} NOT LIKE '-%' AND ${account.handle} NOT LIKE '%-'`,
                postgresql: sql`length(${account.handle}) BETWEEN 1 AND 63 AND (${account.handle} COLLATE "C") !~ '[^a-z0-9-]' AND ${account.handle} NOT LIKE '-%' AND ${account.handle} NOT LIKE '%-'`,
            }),
        ),
    ],
);

/** A persisted account record. */
export type Account = Select<typeof account>;
