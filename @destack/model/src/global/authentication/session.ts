import { check, identifier, index, integer, type Select, sql, table, text } from "@destack/db";
import { device } from "../host/device.ts";
import { user } from "../account/user.ts";

/** Session records. */
export const session = table(
    "session",
    {
        /** The immutable session identifier. */
        id: identifier("id", "session").primaryKey().notNull(),
        /** The authenticated user. */
        userId: identifier("user_id", "user")
            .notNull()
            .references(() => user.id, {
                onDelete: "cascade",
            }),
        /** The registered device, when present. */
        deviceId: identifier("device_id", "device").references(() => device.id, {
            onDelete: "restrict",
        }),
        /** The session credential stored by Better Auth. */
        token: text("token").notNull().unique(),
        /** The session creation time. */
        createdAt: integer("created_at").notNull(),
        /** The last session refresh in epoch milliseconds. */
        updatedAt: integer("updated_at").notNull(),
        /** The client IP observed through trusted ingress. */
        ipAddress: text("ip_address"),
        /** The reported client user agent. */
        userAgent: text("user_agent"),
        /** The approximate country inferred from the observed address. */
        country: text("country"),
        /** The approximate city inferred from the observed address. */
        city: text("city"),
        /** The last completed authentication ceremony. */
        authenticatedAt: integer("authenticated_at"),
        /** The verified method used in the last authentication ceremony. */
        authenticationMethod: text("authentication_method", {
            enum: ["magic-link", "email-otp", "oauth", "device", "totp", "webauthn", "recovery"],
        }),
        /** The session expiry time. */
        expiresAt: integer("expires_at").notNull(),
        /** The time access was revoked. */
        revokedAt: integer("revoked_at"),
    },
    (session) => [
        index("session_user").on(session.userId),
        index("session_device").on(session.deviceId),
        index("session_expiry").on(session.expiresAt),
        check("session_expiry_order", sql`${session.expiresAt} > ${session.createdAt}`),
        check(
            "session_authentication_method",
            sql`${session.authenticationMethod} IS NULL OR ${session.authenticationMethod} IN ('magic-link', 'email-otp', 'oauth', 'device', 'totp', 'webauthn', 'recovery')`,
        ),
        check(
            "session_authentication_time",
            sql`${session.authenticationMethod} IS NULL OR ${session.authenticatedAt} IS NOT NULL`,
        ),
    ],
);

/** A persisted session record. */
export type Session = Select<typeof session>;
