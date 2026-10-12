import { defineSchema, schema } from "@destack/schema";
import { DEVICE_CODE, JWT_BEARER, TOKEN_EXCHANGE } from "./server.ts";

/** The optional token request parameters: scopes (RFC 6749 3.3) and resource (RFC 8707 2). */
const NARROWING = {
    scope: schema.string().min(1).exactOptional(),
    resource: schema.url({ protocol: /^https$/u }).exactOptional(),
};

/** A token request's grant that proves possession of the account's authorization. */
const grant = defineSchema(
    schema.discriminatedUnion("grant_type", [
        schema.object({
            /** An authorization code exchanged with its PKCE verifier (RFC 7636 4.5). */
            grant_type: schema.literal("authorization_code"),
            /** The authorization code. */
            code: schema.string().min(1),
            /** The redirect URI the authorization request named. */
            redirect_uri: schema.httpUrl(),
            /** The PKCE code verifier. */
            code_verifier: schema.string().min(1),
            /** The resource the token is for. */
            resource: NARROWING.resource,
        }),
        schema.object({
            /** A refresh token exchanged for a new access token. */
            grant_type: schema.literal("refresh_token"),
            /** The refresh token. */
            refresh_token: schema.string().min(1),
            ...NARROWING,
        }),
        schema.object({
            /** A device code a person approved. */
            grant_type: schema.literal(DEVICE_CODE),
            /** The device code. */
            device_code: schema.string().min(1),
        }),
        schema.object({
            /** A JWT assertion signed with the connection's key. */
            grant_type: schema.literal(JWT_BEARER),
            /** The assertion. */
            assertion: schema.string().min(1),
            scope: NARROWING.scope,
        }),
        schema.object({
            /** Another connection's token exchanged for this one. */
            grant_type: schema.literal(TOKEN_EXCHANGE),
            /** The token exchanged. */
            subject_token: schema.string().min(1),
            /** The type of the token exchanged. */
            subject_token_type: schema.string().min(1),
            ...NARROWING,
        }),
    ]),
);
/** A token request's grant that proves possession of the account's authorization. */
export type TokenGrant = schema.Infer<typeof grant>;

/** A token request's grant that proves possession, and its form parameters. */
export const TokenGrant = Object.assign(grant, {
    /** Write a grant as the token request's form parameters, leaving out the absent ones. */
    form(granted: TokenGrant): Readonly<Record<string, string>> {
        return Object.fromEntries(
            Object.entries(granted).filter(
                (entry): entry is [string, string] => typeof entry[1] === "string",
            ),
        );
    },
});

/** A token a client asks its server to revoke (RFC 7009 2.1), which proves possession itself. */
export const TokenRevocation = defineSchema(
    schema.object({
        /** The token. */
        token: schema.string().min(1),
        /** Whether the token is a refresh or an access token. */
        hint: schema.enum(["refresh_token", "access_token"]),
    }),
);
/** A token a client asks its authorization server to revoke. */
export type TokenRevocation = schema.Infer<typeof TokenRevocation>;

/** A device authorization request (RFC 8628 3.1). */
export const DeviceRequest = defineSchema(
    schema.object({
        /** The scopes asked for. */
        scopes: schema.array(schema.string().min(1)),
        /** The resource the token will be for (RFC 8707 2). */
        resource: NARROWING.resource,
    }),
);
/** A device authorization request. */
export type DeviceRequest = schema.Infer<typeof DeviceRequest>;
