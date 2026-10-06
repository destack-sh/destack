import { boolean, defineTable, index, integer, text } from "@destack/db";

/** The log of each identity's operations, each signed by a rotation key of the identity before it. */
export const identityOperation = defineTable(
    "identity_operation",
    {
        /** The space or universe the identity is. */
        subject: text("subject").primaryKey(),
        /** The operation's position in the identity's log, from zero. */
        sequence: integer("sequence").primaryKey(),
        /** The universe, where every operation lives. */
        scope: text("scope").notNull(),
        /** The digest of the signed operation. */
        digest: text("digest").notNull(),
        /** The signed operation, a compact JSON Web Signature. */
        operation: text("operation").notNull(),
        /** The priority of the rotation key that signed it among the keys of the identity it follows. */
        priority: integer("priority").notNull(),
        /** When the directory applied it, in UTC epoch milliseconds. */
        appliedAt: integer("applied_at").notNull(),
        /** Whether a higher-priority rotation key nullified it. */
        isNullified: boolean("is_nullified").notNull(),
    },
    {
        log: {},
        constraints: (operation) => [index("identity_operation_digest").on(operation.digest)],
    },
);
