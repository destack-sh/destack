import {
    check,
    identifier,
    index,
    integer,
    json,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { schema } from "@destack/schema";
import { user } from "../account/user.ts";
import { oauthClient } from "./oauth/client.ts";

/** A browser-approved device authorization grant. */
export const deviceAuthorization = table(
    "device_authorization",
    {
        /** The authorization identifier. */
        id: identifier("id", "device-authorization").primaryKey(),
        /** The credential used by the initiating client to poll for approval. */
        deviceCode: text("device_code").notNull().unique(),
        /** The code confirmed by the user in the approving browser. */
        userCode: text("user_code").notNull().unique(),
        /** The approving user. */
        userId: identifier("user_id", "user").references(() => user.id, { onDelete: "cascade" }),
        /** The authorization deadline in epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The browser approval state. */
        status: text("status", { enum: ["pending", "approved", "denied"] }).notNull(),
        /** The most recent token poll in epoch milliseconds. */
        lastPolledAt: integer("last_polled_at"),
        /** The minimum polling interval in milliseconds. */
        pollingInterval: integer("polling_interval"),
        /** The requesting native client. */
        clientId: text("client_id"),
        /** The registered OAuth client receiving tokens from the approved grant. */
        oauthClientId: text("oauth_client_id").references(() => oauthClient.clientId, {
            onDelete: "cascade",
        }),
        /** The OAuth audience URLs approved with the device grant. */
        resources: json("resources", schema.array(schema.string())),
        /** The requested space-separated OAuth scopes. */
        scope: text("scope"),
    },
    (authorization) => [
        index("device_authorization_expiry").on(authorization.expiresAt),
        index("device_authorization_user").on(authorization.userId),
        check(
            "device_authorization_status",
            sql`${authorization.status} IN ('pending', 'approved', 'denied')`,
        ),
    ],
);

/** A persisted device authorization grant. */
export type DeviceAuthorization = Select<typeof deviceAuthorization>;
