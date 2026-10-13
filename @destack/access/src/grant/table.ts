import { check, defineTable, identifier, index, integer, json, text } from "@destack/db";
import { defineSchema, schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { GrantPermission } from "./permission.ts";

/** How often a grant asks its delegator to approve a call: once when created, or for each call. */
export const GRANT_APPROVALS = ["once", "each"] as const;

/** How often a grant asks its delegator to approve a call. */
export const GrantApproval = defineSchema(schema.enum(GRANT_APPROVALS));
/** How often a grant asks its delegator to approve a call. */
export type GrantApproval = schema.Infer<typeof GrantApproval>;

/** A delegator's grant of permissions to a delegate acting for it, in the delegator's scope. */
export const accessGrant = defineTable(
    "grant",
    {
        /** The immutable grant identifier. */
        id: identifier("id", "grant").primaryKey(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The revision that conditional revocations compare against. */
        revision: integer("revision").notNull().default(1),
        /** The delegator's scope the grant lives in and routes changes to. */
        scope: text("scope").notNull(),
        /** The principal whose authority the grant gives. */
        delegator: json("delegator", Subject).notNull(),
        /** The delegator's key, listing a principal's grants. */
        delegatorKey: text("delegator_key").notNull(),
        /** The principal acting for the delegator. */
        delegate: json("delegate", Subject).notNull(),
        /** The delegate's key, listing the grants a principal acts under. */
        delegateKey: text("delegate_key").notNull(),
        /** The delegate's scope, listing the grants to the principals of a scope. */
        delegateScope: text("delegate_scope").notNull(),
        /** The permissions the delegate may use for the delegator. */
        permissions: json("permissions", schema.array(GrantPermission.schema)).notNull(),
        /** How often the delegator approves a call. */
        approval: text("approval").validate(GrantApproval).notNull(),
        /** The exclusive time the grant lapses, in Unix milliseconds, null for none. */
        expiresAt: integer("expires_at"),
    },
    {
        log: { retention: "history" },
        constraints: (grant) => [
            index("grant_delegator").on(grant.delegatorKey),
            index("grant_delegate").on(grant.delegateKey),
            index("grant_delegate_scope").on(grant.delegateScope),
            check("grant_approval", { approval: { in: [...GRANT_APPROVALS] } }),
            check("grant_expiry", {
                OR: [
                    { expiresAt: { isNull: true } },
                    { expiresAt: { gt: { field: "createdAt" } } },
                ],
            }),
        ],
    },
);
