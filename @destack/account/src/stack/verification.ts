import { identifier, index, integer, type Select, defineTable, text } from "@destack/db";

/** An expiring authentication challenge stored by the account service. */
export const verification = defineTable(
    "verification",
    {
        /** The challenge identifier. */
        id: identifier("id", "verification").primaryKey(),
        /** The challenge purpose and subject. */
        identifier: text("identifier").notNull(),
        /** The protected challenge value in the authentication provider's format. */
        value: text("value").notNull(),
        /** The expiry in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The creation time in epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
        /** The last update in epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
    },
    {
        tier: "global",
        constraints: (challenge) => [
            index("verification_identifier").on(challenge.identifier),
            index("verification_expiry").on(challenge.expiresAt),
        ],
    },
);

/** A shared authentication rate-limit counter. */
export const rateLimit = defineTable(
    "rate_limit",
    {
        /** The counter identifier. */
        id: identifier("id", "rate-limit").primaryKey(),
        /** The request bucket selected by Better Auth. */
        key: text("key").notNull().unique(),
        /** The number of requests in the current window. */
        count: integer("count").notNull(),
        /** The most recent request in epoch milliseconds. */
        lastRequest: integer("last_request").notNull(),
    },
    {
        tier: "global",
        constraints: (limit) => [index("rate_limit_last_request").on(limit.lastRequest)],
    },
);

/** A persisted authentication challenge. */
export type Verification = Select<typeof verification>;
/** A persisted authentication rate-limit counter. */
export type RateLimit = Select<typeof rateLimit>;
