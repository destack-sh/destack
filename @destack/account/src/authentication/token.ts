import { defineErrorFamily, type FamilyError, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import { AccountError, type AccountReason } from "../error/error.ts";

/** The milliseconds in a second, as OAuth and JWT count times and durations in seconds. */
export const SECOND_MILLISECONDS = 1000;

/** The media type of a JSON answer (RFC 8259 11). */
const JSON_TYPE = "application/json";

/** The OAuth refusals, each reason the wire code its RFC spells, transcribed in upper case. */
export const OAuthError = defineErrorFamily(
    { name: "oauth" },
    {
        // the token endpoint's (RFC 6749 5.2, RFC 8707 2, RFC 8628 3.5)
        INVALID_REQUEST: { code: "BAD_REQUEST" },
        INVALID_CLIENT: { code: "UNAUTHORIZED" },
        INVALID_GRANT: { code: "BAD_REQUEST" },
        UNAUTHORIZED_CLIENT: { code: "BAD_REQUEST" },
        UNSUPPORTED_GRANT_TYPE: { code: "BAD_REQUEST" },
        INVALID_SCOPE: { code: "BAD_REQUEST" },
        INVALID_TARGET: { code: "BAD_REQUEST" },
        AUTHORIZATION_PENDING: { code: "BAD_REQUEST" },
        SLOW_DOWN: { code: "BAD_REQUEST" },
        ACCESS_DENIED: { code: "BAD_REQUEST" },
        EXPIRED_TOKEN: { code: "BAD_REQUEST" },

        // the authorization endpoint's (RFC 6749 4.1.2.1, OIDC Core 3.1.2.6, RFC 9126 2.2)
        UNSUPPORTED_RESPONSE_TYPE: { code: "BAD_REQUEST" },
        LOGIN_REQUIRED: { code: "BAD_REQUEST" },
        CONSENT_REQUIRED: { code: "BAD_REQUEST" },
        INVALID_REQUEST_URI: { code: "BAD_REQUEST" },

        // the resource endpoints' (RFC 6750 3.1, RFC 7009 2.2.1)
        INVALID_TOKEN: { code: "UNAUTHORIZED" },
        UNSUPPORTED_TOKEN_TYPE: { code: "BAD_REQUEST" },
    },
);
/** An OAuth refusal's reason. */
export type OAuthReason = ReasonOf<typeof OAuthError>;

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
                oauth?.success === true
                    ? OAuthError.reason.safeParse(oauth.data.error.toUpperCase())
                    : undefined;

            // fail as a bad gateway on an answer without a known OAuth error
            if (oauth?.success !== true || reason?.success !== true) {
                return AccountError.of(
                    "TOKEN_ENDPOINT_FAILED",
                    `the token endpoint responded with ${response.status}: ${text}`,
                );
            }

            // refuse with the OAuth error and its description
            const { error, error_description: description } = oauth.data;

            return OAuthError.of(
                reason.data,
                description === undefined
                    ? `the token endpoint refused the request: ${error}`
                    : `the token endpoint refused the request: ${error}: ${description}`,
            );
        },
    },
);
/** A token endpoint's answer. */
export type TokenResponse = schema.Infer<typeof TokenResponse>;
