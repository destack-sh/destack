import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** An authorization server's refusal: its OAuth error code (RFC 6749 5.2). */
const oauthRefusal = schema.object({
    /** The OAuth error code, such as invalid_grant or authorization_pending. */
    error: schema.string(),
});

/** An authorization server's refusal, and the code a failure carries of one. */
export const OAuthRefusal = Object.assign(oauthRefusal, {
    /** Read the OAuth error code a server refused with, absent for other failures. */
    code(error: unknown): string | undefined {
        return ConnectionError.is(error, "AUTHORIZATION_UNREACHABLE")
            ? error.data.error
            : undefined;
    },
});
/** An authorization server's refusal. */
export type OAuthRefusal = schema.Infer<typeof oauthRefusal>;

/** The connection package's failures, each reason with the call error code it answers. */
export const ConnectionError = defineErrorFamily(import.meta.destack.package, {
    // not found
    AUTHORIZATION_NOT_FOUND: {},
    CONNECTION_NOT_FOUND: {},
    INSTALLATION_PAGE_NOT_FOUND: {},
    INTEGRATION_NOT_FOUND: {},

    // invalid
    AUTHORIZATION_INVALID: {},
    MATERIAL_INVALID: {},
    REDIRECT_URI_INVALID: {},
    SECRET_INVALID: {},
    SIGNING_KEY_INVALID: {},

    // required
    CLIENT_SECRET_REQUIRED: {},
    INTEGRATION_REQUIRED: {},
    REDIRECT_URI_REQUIRED: {},

    // denied
    EXCHANGE_SOURCE_DENIED: {},
    PRIMARY_DENIED: {},
    SESSION_DENIED: {},

    // blocked
    CONNECTION_BLOCKED: {},
    CREDENTIAL_BLOCKED: {},
    ISSUER_BLOCKED: {},
    REFRESH_TOKEN_BLOCKED: {},

    // unsupported
    ENDPOINT_UNSUPPORTED: {},
    REGISTRATION_UNSUPPORTED: {},

    // unavailable
    TOKEN_UNAVAILABLE: {},

    // unreachable
    AUTHORIZATION_UNREACHABLE: { data: OAuthRefusal },
    RESPONSE_UNREACHABLE: {},
    SERVER_UNREACHABLE: {},
});

/** A stable cause of connection failures. */
export type ConnectionReason = ReasonOf<typeof ConnectionError>;
