import { boolean, defineTable, json, text, textEnum } from "@destack/db";
import * as sync from "@destack/sync";

/** The replicas a store's subscriber asks for, each followed from its home or dropped. */
export const replicaRequest = defineTable(
    "replica_request",
    {
        /** The replica's key: its subscription's name and scope. */
        key: text("key").primaryKey(),
        /** The scope whose rows the replica copies, which files the request's changes. */
        scope: text("scope").notNull(),
        /** Follow the replica, or drop a stored one no request asks for. */
        action: textEnum("action", ["follow", "drop"]).notNull(),
        /** The subscription following or recorded for the replica. */
        subscription: json("subscription", sync.SubscriptionRecord).notNull(),
        /** The scope whose publisher streams a followed replica, null for a drop. */
        home: text("home"),
        /** Whether a use acquired the replica's key, not the store's declaration. */
        isAcquired: boolean("is_acquired").notNull(),
        /** The digest of the action and subscription, which a listing compares. */
        digest: text("digest").notNull(),
    },
    { log: {} },
);
