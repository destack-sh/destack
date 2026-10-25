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
        return ConnectionError.is(error, "AUTHORIZATION_REFUSED") ? error.data.error : undefined;
    },
});
/** An authorization server's refusal. */
export type OAuthRefusal = schema.Infer<typeof oauthRefusal>;

/** The connection package's failures, each reason with the call error code it answers. */
export const ConnectionError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    AUTHORIZATION_INVALID: { code: "BAD_REQUEST" },
    AUTHORIZATION_UNSUPPORTED: { code: "BAD_REQUEST" },
    INTEGRATION_REQUIRED: { code: "BAD_REQUEST" },
    MATERIAL_MISMATCH: { code: "BAD_REQUEST" },
    REDIRECT_URI_REQUIRED: { code: "BAD_REQUEST" },
    REDIRECT_URI_UNREGISTERED: { code: "BAD_REQUEST" },
    SECRET_UNSUPPORTED: { code: "BAD_REQUEST" },
    SECRET_VAULT_MISMATCH: { code: "BAD_REQUEST" },
    SIGNING_KEY_INVALID: { code: "BAD_REQUEST" },
    VAULT_FOREIGN: { code: "BAD_REQUEST" },

    // forbidden
    EXCHANGE_SOURCE_FOREIGN: { code: "FORBIDDEN" },
    PRIMARY_REQUIRED: { code: "FORBIDDEN" },
    SESSION_MISMATCH: { code: "FORBIDDEN" },
    SESSION_REQUIRED: { code: "FORBIDDEN" },

    // not found
    AUTHORIZATION_NOT_FOUND: { code: "NOT_FOUND" },
    CONNECTION_NOT_FOUND: { code: "NOT_FOUND" },
    INTEGRATION_NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    CLIENT_SECRET_MISSING: { code: "CONFLICT" },
    CONNECTION_NOT_AUTHORIZED: { code: "CONFLICT" },
    CONNECTION_NOT_INSTALLED: { code: "CONFLICT" },
    CONNECTION_REVOKED: { code: "CONFLICT" },
    CREDENTIAL_MISSING: { code: "CONFLICT" },
    ENDPOINT_MISSING: { code: "CONFLICT" },
    INSTALLATION_PAGE_MISSING: { code: "CONFLICT" },
    ISSUER_NOT_INSTALLATION: { code: "CONFLICT" },
    ISSUER_NOT_SERVER: { code: "CONFLICT" },
    REFRESH_TOKEN_MISSING: { code: "CONFLICT" },
    REGISTRATION_UNSUPPORTED: { code: "CONFLICT" },
    VAULT_REQUIRED: { code: "CONFLICT" },

    // bad gateway
    AUTHORIZATION_REFUSED: { code: "BAD_GATEWAY", data: OAuthRefusal },
    RESPONSE_INVALID: { code: "BAD_GATEWAY" },
    SERVER_FAILED: { code: "BAD_GATEWAY" },
    SERVER_UNREACHABLE: { code: "BAD_GATEWAY" },

    // service unavailable
    TOKEN_EXPIRED: { code: "SERVICE_UNAVAILABLE" },
    TOKEN_PENDING: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of connection failures. */
export type ConnectionReason = ReasonOf<typeof ConnectionError>;
