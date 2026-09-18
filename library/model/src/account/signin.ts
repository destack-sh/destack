import {
    check,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { user } from "./user.ts";

/** A short-lived native-client sign-in request completed through a browser. */
export const signInRequest = table("sign_in_request", {
    ...recordColumns("sign-in-request"),
    /** The requesting Destack client. */
    client: text("client", { enum: ["desktop", "cli"] }).notNull(),
    /** The hash of the secret used to poll or exchange this request. */
    tokenHash: text("token_hash").notNull().unique(),
    /** The hash of the user-visible approval code. */
    codeHash: text("code_hash").notNull().unique(),
    /** The PKCE S256 challenge bound to the initiating client. */
    challenge: text("challenge").notNull(),
    /** The authenticated approving user. */
    userId: identifier("user_id", "user").references(() => user.id),
    /** The request expiration. */
    expiresAt: integer("expires_at").notNull(),
    /** Browser approval time. */
    approvedAt: integer("approved_at"),
    /** Browser denial time. */
    deniedAt: integer("denied_at"),
    /** The time a single credential exchange consumed this request. */
    consumedAt: integer("consumed_at"),
}, (request) => [
    check("sign_in_request_client", sql`${request.client} IN ('desktop', 'cli')`),
    check("sign_in_request_expiry", sql`${request.expiresAt} > ${request.createdAt}`),
    check(
        "sign_in_request_approval",
        sql`(${request.approvedAt} IS NULL) = (${request.userId} IS NULL) AND (${request.approvedAt} IS NULL OR (${request.deniedAt} IS NULL AND ${request.approvedAt} >= ${request.createdAt} AND ${request.approvedAt} < ${request.expiresAt}))`,
    ),
    check(
        "sign_in_request_exchange",
        sql`${request.consumedAt} IS NULL OR (${request.approvedAt} IS NOT NULL AND ${request.consumedAt} >= ${request.approvedAt} AND ${request.consumedAt} < ${request.expiresAt})`,
    ),
]);

/** A native-client sign-in request. */
export type SignInRequest = Select<typeof signInRequest>;
