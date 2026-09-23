import { identifier, integer, json, table, text } from "@destack/db";
import { Login } from "../service/login.ts";

/** Retained account sessions; secret values remain in secure host credential storage. */
export const login = table("login", {
    /** Local selection identity retained across issuer session renewal. */
    id: identifier("id", "login").primaryKey(),
    /** Universe authentication issuer. */
    issuer: text("issuer").notNull(),
    /** Last verified remote session. */
    sessionId: identifier("session_id", "session"),
    /** Verified universe user. */
    subject: json("subject", Login.shape.subject).notNull(),
    /** Last verified display name. */
    name: text("name").notNull(),
    /** Opaque OS credential reference, never the session token itself. */
    credential: text("credential").notNull(),
    /** Whether interactive reauthentication is required. */
    status: text("status", { enum: ["authenticated", "reauthentication"] }).notNull(),
    /** Last successful issuer contact, in UTC milliseconds. */
    verifiedAt: integer("verified_at"),
});
