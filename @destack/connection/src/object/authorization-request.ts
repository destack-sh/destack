import { principal, through } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Duration, schema } from "@destack/schema";
import { connection } from "./connection.ts";

/** How long an authorization waits for the person: ten minutes. */
const AUTHORIZATION_WINDOW: Duration = { minutes: 10 };

/** How a person authorizes a connection: on an authorization, installation or device page. */
export const AUTHORIZATION_KINDS = ["page", "installation", "device"] as const;

/** A pending authorization of a connection's external account, used once (RFC 9700 4.7). */
export const authorizationRequest = defineObject({
    name: "authorization-request",
    plural: "authorizationRequests",
    nested: { in: connection, delete: "cascade", receive: "manage" },
    expiring: [{ after: AUTHORIZATION_WINDOW, from: "createdAt" }],
    fields: {
        /** How the person authorizes: on an authorization, installation or device page. */
        kind: field.enum(AUTHORIZATION_KINDS),
        /** The person authorizing the external account. */
        user: field.reference(principal.user),
        /** The session the person started the authorization in, which completes it. */
        session: field.string(schema.string().min(1)).sensitive(),

        // a page or installation authorization
        /** The state the authorization server returns, for a page or installation authorization. */
        state: field.string(schema.string().min(1)).optional(),
        /** The URI the server redirects the person back to, for a page or installation. */
        redirectUri: field.string(schema.httpUrl()).optional(),
        /** The PKCE code verifier of the code exchange (RFC 7636), for a page authorization. */
        verifier: field.string(schema.string().min(1)).sensitive().optional(),

        // a device authorization
        /** The device code the controller polls the token endpoint with (RFC 8628 3.4). */
        deviceCode: field.string(schema.string().min(1)).sensitive().optional(),
        /** The seconds between polls the authorization server asks for (RFC 8628 3.2). */
        interval: field.integer().optional(),
    },
    // the connection's managers start and use its authorizations
    permissions: { read: through("parent", "read"), record: through("parent", "manage") },
    constraints: {
        state: { kind: "unique", on: ["state"] },
        kind: {
            kind: "check",
            condition: {
                OR: [
                    {
                        kind: "page",
                        state: { isNotNull: true },
                        redirectUri: { isNotNull: true },
                        verifier: { isNotNull: true },
                        deviceCode: { isNull: true },
                        interval: { isNull: true },
                    },
                    {
                        kind: "installation",
                        state: { isNotNull: true },
                        redirectUri: { isNotNull: true },
                        verifier: { isNull: true },
                        deviceCode: { isNull: true },
                        interval: { isNull: true },
                    },
                    {
                        kind: "device",
                        deviceCode: { isNotNull: true },
                        interval: { isNotNull: true },
                        state: { isNull: true },
                        redirectUri: { isNull: true },
                        verifier: { isNull: true },
                    },
                ],
            },
        },
    },
}).methods((method) => ({
    /** Record a started authorization of one kind, as the connection package. */
    start: method.create("record", {
        isInternal: true,
        fields: ["userId", "session"],
        variants: {
            kind: {
                page: ["state", "redirectUri", "verifier"],
                installation: ["state", "redirectUri"],
                device: ["deviceCode", "interval"],
            },
        },
    }),
    /** Use the authorization once, as the connection package. */
    delete: method.delete("record", { isInternal: true }),
}));
/** A pending authorization of a connection. */
export type AuthorizationRequest = Select<typeof authorizationRequest.table>;
