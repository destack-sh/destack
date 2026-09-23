import { identifier, integer, table, text } from "@destack/db";

/** Accepted regional execution instructions retained through daemon restart. */
export const deployment = table("deployment", {
    /** Immutable regional deployment identity. */
    id: identifier("id", "deployment").primaryKey(),
    /** Space for filtered host administration. */
    spaceId: identifier("space_id", "space").notNull(),
    /** Accepted installation generation. */
    generation: integer("generation").notNull(),
    /** Verified execution-host authorization epoch. */
    hostEpoch: integer("host_epoch").notNull(),
    /** Exclusive authorization expiry, in UTC milliseconds. */
    authorizedUntil: integer("authorized_until").notNull(),
    /** Desired execution availability. */
    status: text("status", { enum: ["active", "draining", "stopped"] }).notNull(),
    /** Desired concurrent instance count on this host. */
    instances: integer("instances").notNull(),
    /** Authority-selected exit behavior. */
    restart: text("restart", { enum: ["never", "on-failure", "always"] }).notNull(),
    /** Last authority verification, in UTC milliseconds. */
    verifiedAt: integer("verified_at").notNull(),
});
