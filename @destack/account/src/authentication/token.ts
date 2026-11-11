import { defineErrorFamily, type FamilyError, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import { AccountError, type AccountReason } from "../error/error.ts";
import { GrantDetail } from "./request.ts";

/** The milliseconds in a second, as OAuth and JWT count times and durations in seconds. */
export const SECOND_MILLISECONDS = 1000;

/** The media type of a JSON answer (RFC 8259 11). */
const JSON_TYPE = "application/json";

/** The JWT type of a request object (RFC 9101 10.8). */
export const REQUEST_OBJECT_TYPE = "oauth-authz-req+jwt";

/** What a person must grant before a token carries it: the grant an installation asks, if any. */
export const ConsentRequest = schema.object({
    /** The part of the installation's grant no live grant gives, none for a sign-in's consent. */
    authorization_details: schema.array(GrantDetail).max(1),
});
/** The permissions a person must grant before a delegate's token carries them. */
export type ConsentRequest = schema.Infer<typeof ConsentRequest>;

/** The OAuth refusals, each reason the wire code its RFC spells, transcribed in upper case. */
export const OAuthError = defineErrorFamily(
    { name: "oauth" },
    {
        // invalid
        AUTHENTICATION_INVALID: {},
        AUTHORIZATION_INVALID: {},
        CLIENT_INVALID: {},
        CONSENT_INVALID: {},
        GRANT_INVALID: {},
        GRANT_TYPE_INVALID: {},
        POLLING_INVALID: {},
        REQUEST_INVALID: {},
        REQUEST_OBJECT_INVALID: {},
        REQUEST_URI_INVALID: {},
        RESPONSE_TYPE_INVALID: {},
        SCOPE_INVALID: {},
        TARGET_INVALID: {},
        TOKEN_INVALID: {},
        TOKEN_TYPE_INVALID: {},

        // required
        CONSENT_REQUIRED: { data: ConsentRequest },
        LOGIN_REQUIRED: {},

        // unauthenticated
        CLIENT_UNAUTHENTICATED: {},
        TOKEN_UNAUTHENTICATED: {},
    },
);
/** An OAuth refusal's reason. */
export type OAuthReason = ReasonOf<typeof OAuthError>;

/** The wire code of each OAuth refusal, as its RFC or OpenID Connect spells it. */
export const OAUTH_ERROR_CODES = {
    AUTHENTICATION_INVALID: "unmet_authentication_requirements",
    AUTHORIZATION_INVALID: "authorization_pending",
    CLIENT_INVALID: "unauthorized_client",
    CLIENT_UNAUTHENTICATED: "invalid_client",
    CONSENT_INVALID: "access_denied",
    CONSENT_REQUIRED: "consent_required",
    GRANT_INVALID: "invalid_grant",
    GRANT_TYPE_INVALID: "unsupported_grant_type",
    LOGIN_REQUIRED: "login_required",
    POLLING_INVALID: "slow_down",
    REQUEST_INVALID: "invalid_request",
    REQUEST_OBJECT_INVALID: "invalid_request_object",
    REQUEST_URI_INVALID: "invalid_request_uri",
    RESPONSE_TYPE_INVALID: "unsupported_response_type",
    SCOPE_INVALID: "invalid_scope",
    TARGET_INVALID: "invalid_target",
    TOKEN_INVALID: "expired_token",
    TOKEN_TYPE_INVALID: "unsupported_token_type",
    TOKEN_UNAUTHENTICATED: "invalid_token",
} as const satisfies Readonly<Record<OAuthReason, string>>;

/** Read OAuth wire codes. */
export const OAuthErrorCode = {
    /** Read the refusal a wire code spells, absent for a code no refusal has. */
    reason(code: string): OAuthReason | undefined {
        const found = Object.entries(OAUTH_ERROR_CODES).find(([, wire]) => wire === code);
        const parsed = OAuthError.reason.safeParse(found?.[0]);

        return parsed.success ? parsed.data : undefined;
    },
};

/** A refused OAuth request (RFC 6749 5.2, RFC 8628 3.5). */
export const ErrorResponse = schema.looseObject({
    /** The error code. */
    error: schema.string().min(1),
    /** What the error means, for the person (RFC 6749 5.2). */
    error_description: schema.string().exactOptional(),
});

/** A token endpoint's answer (RFC 6749 5.1). */
export const TokenResponse = Object.assign(
    schema.looseObject({
        /** The access token. */
        access_token: schema.string().min(1),
        /** The access token's type, which Destack issues as bearer tokens (RFC 6750). */
        token_type: schema.string().regex(/^bearer$/iu),
        /** The seconds the access token lasts from the answer. */
        expires_in: schema.number().int().nonnegative(),
        /** The refresh token, granted for offline access. */
        refresh_token: schema.string().min(1).exactOptional(),
        /** The ID token identifying the signed-in person, granted with the openid scope. */
        id_token: schema.string().min(1).exactOptional(),
        /** The granted scopes, space-delimited (RFC 6749 5.1). */
        scope: schema.string().exactOptional(),
    }),
    {
        /** Read the granted access token and when it lapses, counted from its receipt now. */
        granted(
            response: { readonly access_token: string; readonly expires_in: number },
            now: number,
        ): { readonly accessToken: string; readonly expiresAt: number } {
            return {
                accessToken: response.access_token,
                expiresAt: now + response.expires_in * SECOND_MILLISECONDS,
            };
        },

        /** Read an OAuth endpoint's refusal as its OAuth error, else as a bad gateway. */
        async refusal(response: Response): Promise<FamilyError<AccountReason | OAuthReason>> {
            // read the body, and the OAuth error of a JSON one
            const text = await response.text();
            const isJson = response.headers.get("content-type")?.startsWith(JSON_TYPE) === true;
            const oauth = isJson ? ErrorResponse.safeParse(JSON.parse(text)) : undefined;
            const reason =
                oauth?.success === true ? OAuthErrorCode.reason(oauth.data.error) : undefined;

            // fail as a bad gateway on an answer without a known OAuth error
            if (oauth?.success !== true || reason === undefined) {
                return AccountError.of(
                    "TOKEN_ENDPOINT_UNREACHABLE",
                    `could not call the token endpoint: ${response.status} '${text}'`,
                );
            }

            // refuse with the OAuth error and its description, a consent with the permissions it needs
            const { error, error_description: description } = oauth.data;
            const message =
                description === undefined
                    ? `the token endpoint refused the request: ${error}`
                    : `the token endpoint refused the request: ${error}: ${description}`;

            return reason === "CONSENT_REQUIRED"
                ? OAuthError.of("CONSENT_REQUIRED", message, {
                      data: ConsentRequest.parse({
                          authorization_details: oauth.data["authorization_details"],
                      }),
                  })
                : OAuthError.of(reason, message);
        },
    },
);
/** A token endpoint's answer. */
export type TokenResponse = schema.Infer<typeof TokenResponse>;
