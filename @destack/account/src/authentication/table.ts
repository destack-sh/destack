import {
    type Controller,
    type DatabaseConnection,
    defineTable,
    identifier,
    inArray,
    index,
    integer,
    now,
    type Select,
    text,
} from "@destack/db";

/** An expiring authentication challenge stored by the universe. */
export const verification = defineTable(
    "verification",
    {
        /** The challenge's key. */
        id: text("id").primaryKey(),
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
        constraints: (challenge) => [
            index("verification_identifier").on(challenge.identifier),
            index("verification_expiry").on(challenge.expiresAt),
        ],
    },
);

/** Authentication challenges, removed once they lapse. */
export const Verification = {
    /** Declare the controller removing lapsed challenges, a page at a time. */
    controller(database: DatabaseConnection): Controller<typeof verification> {
        return {
            name: "verification-expiry",
            table: verification,
            pending: { expiresAt: { lte: now } },
            key: [],
            reconcile: async (rows) => {
                await database.transaction(
                    async (transaction) =>
                        await transaction.delete(verification).where(
                            inArray(
                                verification.id,
                                rows.map((row) => row.id),
                            ),
                        ),
                );
            },
        };
    },
};

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
        constraints: (limit) => [index("rate_limit_last_request").on(limit.lastRequest)],
    },
);

/** A persisted authentication challenge. */
export type Verification = Select<typeof verification>;
/** A persisted authentication rate-limit counter. */
export type RateLimit = Select<typeof rateLimit>;
