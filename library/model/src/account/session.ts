import { check, identifier, index, integer, type Select, sql, table, text } from "@destack/db";
import { device } from "../host/device.ts";
import { user } from "./user.ts";

/** Session records. */
export const session = table("session", {
    /** The immutable session identifier. */
    id: identifier("id", "session").primaryKey().notNull(),
    /** The authenticated user. */
    userId: identifier("user_id", "user").notNull().references(() => user.id, {
        onDelete: "cascade",
    }),
    /** The registered device, when present. */
    deviceId: identifier("device_id", "device").references(() => device.id, {
        onDelete: "restrict",
    }),
    /** The hash of the session credential. */
    tokenHash: text("token_hash").notNull().unique(),
    /** The session creation time. */
    createdAt: integer("created_at").notNull(),
    /** The session expiry time. */
    expiresAt: integer("expires_at").notNull(),
    /** The time access was revoked. */
    revokedAt: integer("revoked_at"),
}, (session) => [
    index("session_user").on(session.userId),
    index("session_expiry").on(session.expiresAt),
    check("session_expiry_order", sql`${session.expiresAt} > ${session.createdAt}`),
]);

/** A persisted session record. */
export type Session = Select<typeof session>;
