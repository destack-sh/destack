import { principal, through } from "@destack/access";
import { check, type Select, sql, unique } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Duration, schema } from "@destack/schema";
import { connection } from "./connection.ts";

/** How long an authorization waits for the person, ten minutes as common authorization codes and RFC 8628's device codes last. */
const AUTHORIZATION_WINDOW: Duration = { minutes: 10 };

/** How a person authorizes a connection: on an authorization page, on an integration's installation page, or on a device's verification page (RFC 8628). */
export const AUTHORIZATION_KINDS = ["page", "installation", "device"] as const;

/** A pending authorization of a connection's external account, bound to the person and session that started it and used once (RFC 9700 4.7). */
export const authorizationRequest = defineObject({
    name: "authorization-request",
    plural: "authorizationRequests",
    nested: { in: connection, delete: "cascade", receive: "manage" },
    expiring: [{ after: AUTHORIZATION_WINDOW, from: "createdAt" }],
    fields: {
        /** How the person authorizes: on an authorization page, on an installation page, or on a device's verification page. */
        kind: field.enum(AUTHORIZATION_KINDS),
        /** The person authorizing the external account. */
        user: field.reference(principal.user),
        /** The session the person started the authorization in, which completes it. */
        session: field.string(schema.string().min(1)).sensitive(),

        // a page or installation authorization
        /** The state the authorization server returns, for a page or installation authorization. */
        state: field.string(schema.string().min(1)).optional(),
        /** The URI the authorization server redirects the person back to, for a page or installation authorization. */
        redirectUri: field.string(schema.httpUrl()).optional(),
        /** The PKCE code verifier the code exchange presents (RFC 7636), for a page authorization. */
        verifier: field.string(schema.string().min(1)).sensitive().optional(),

        // a device authorization
        /** The device code the controller polls the token endpoint with (RFC 8628 3.4). */
        deviceCode: field.string(schema.string().min(1)).sensitive().optional(),
        /** The seconds between polls the authorization server asks for (RFC 8628 3.2). */
        interval: field.integer().optional(),
    },
    constraints: (entry) => [
        unique("authorization_request_state").on(entry.state),
        check(
            "authorization_request_kind",
            sql`(${entry.kind} = 'page' AND ${entry.state} IS NOT NULL AND ${entry.redirectUri} IS NOT NULL AND ${entry.verifier} IS NOT NULL AND ${entry.deviceCode} IS NULL AND ${entry.interval} IS NULL)
                OR (${entry.kind} = 'installation' AND ${entry.state} IS NOT NULL AND ${entry.redirectUri} IS NOT NULL AND ${entry.verifier} IS NULL AND ${entry.deviceCode} IS NULL AND ${entry.interval} IS NULL)
                OR (${entry.kind} = 'device' AND ${entry.deviceCode} IS NOT NULL AND ${entry.interval} IS NOT NULL AND ${entry.state} IS NULL AND ${entry.redirectUri} IS NULL AND ${entry.verifier} IS NULL)`,
        ),
    ],
    // the connection's managers start and use its authorizations
    permissions: { read: through("parent", "read"), record: through("parent", "manage") },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        /** Record a started authorization of one kind, as its connection's service. */
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
        /** Use the authorization once, as its connection's service. */
        delete: method.delete("record", { isInternal: true }),
    }),
});
/** A pending authorization of a connection. */
export type AuthorizationRequest = Select<typeof authorizationRequest.table>;
