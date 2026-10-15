import { check, defineTable, identifier, index, integer, json, text } from "@destack/db";
import { schema } from "@destack/schema";
import { GrantPermission } from "./permission.ts";

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
        /** The package of the principal whose authority the grant gives. */
        delegatorPackageId: identifier("delegator_package_id", "package").notNull(),
        /** The type of the delegator. */
        delegatorType: text("delegator_type").notNull(),
        /** The scope of the delegator. */
        delegatorScope: text("delegator_scope").notNull(),
        /** The identifier of the delegator. */
        delegatorId: text("delegator_id").notNull(),
        /** The package of the principal acting for the delegator. */
        delegatePackageId: identifier("delegate_package_id", "package").notNull(),
        /** The type of the delegate. */
        delegateType: text("delegate_type").notNull(),
        /** The scope of the delegate, listing the grants to the principals of a scope. */
        delegateScope: text("delegate_scope").notNull(),
        /** The identifier of the delegate. */
        delegateId: text("delegate_id").notNull(),
        /** The permissions the delegate may use for the delegator. */
        permissions: json("permissions", schema.array(GrantPermission.schema)).notNull(),
        /** The exclusive time the grant lapses, in Unix milliseconds, null for none. */
        expiresAt: integer("expires_at"),
    },
    {
        log: { retention: "history" },
        constraints: (grant) => [
            index("grant_delegator").on(
                grant.delegatorPackageId,
                grant.delegatorType,
                grant.delegatorScope,
                grant.delegatorId,
            ),
            index("grant_delegate").on(
                grant.delegateScope,
                grant.delegatePackageId,
                grant.delegateType,
                grant.delegateId,
            ),
            check("grant_expiry", {
                OR: [
                    { expiresAt: { isNull: true } },
                    { expiresAt: { gt: { field: "createdAt" } } },
                ],
            }),
        ],
    },
);
