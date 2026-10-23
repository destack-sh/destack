import { CallError } from "@destack/object";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";
import type { FamilyError } from "@destack/error";
import { AccountError, type AccountReason } from "../error/error.ts";

/** The milliseconds in a second, as OAuth counts expiries in seconds. */
const SECOND_MILLISECONDS = 1000;

/** The domain of the token errors RFC 6749 section 5.2 defines, such as INVALID_GRANT. */
const OAUTH_DOMAIN = "oauth";

/** The call error of each status a token endpoint refuses with an OAuth error (RFC 6749 5.2). */
const REFUSALS: ReadonlyMap<number, "BAD_REQUEST" | "UNAUTHORIZED" | "FORBIDDEN"> = new Map([
    [400, "BAD_REQUEST"],
    [401, "UNAUTHORIZED"],
    [403, "FORBIDDEN"],
]);

/** A refused token request (RFC 6749 5.2, RFC 8628 3.5). */
export const ErrorResponse = schema.looseObject({
    /** The error code. */
    error: schema.string().min(1),
    /** What the error means, for the person (RFC 6749 5.2). */
    error_description: schema.string().exactOptional(),
});

/** A token endpoint's answer (RFC 6749 5.1), with the provider's absolute expiry. */
export const TokenResponse = Object.assign(
    schema.looseObject({
        /** The access token. */
        access_token: schema.string().min(1),
        /** When the access token expires, in UTC epoch seconds. */
        expires_at: schema.number().int(),
        /** The refresh token, granted for offline access. */
        refresh_token: schema.string().min(1).exactOptional(),
        /** The ID token identifying the signed-in person, granted with the openid scope. */
        id_token: schema.string().min(1).exactOptional(),
    }),
    {
        /** Read the granted access token and when it lapses, in UTC epoch milliseconds. */
        granted(response: { readonly access_token: string; readonly expires_at: number }): {
            readonly accessToken: string;
            readonly expiresAt: number;
        } {
            return {
                accessToken: response.access_token,
                expiresAt: response.expires_at * SECOND_MILLISECONDS,
            };
        },

        /** Read a token endpoint's refusal as a call error by its status, with its OAuth error. */
        async refusal(
            response: Response,
        ): Promise<FamilyError<AccountReason> | CallError<string, unknown>> {
            // fail as a bad gateway on an answer without an RFC 6749 error, such as a failure
            const code = REFUSALS.get(response.status);
            if (code === undefined) {
                return AccountError.of(
                    "TOKEN_ENDPOINT_FAILED",
                    `the token endpoint responded with ${response.status}: ${await response.text()}`,
                );
            }

            // refuse as the status says, with the OAuth error the endpoint answers
            const { error, error_description: description } = ErrorResponse.parse(
                await response.json(),
            );

            return new CallError(code, {
                reason: error.toUpperCase(),
                domain: OAUTH_DOMAIN,
                message:
                    description === undefined
                        ? `the token endpoint refused the request: ${error}`
                        : `the token endpoint refused the request: ${error}: ${description}`,
            });
        },
    },
);
/** A token endpoint's answer. */
export type TokenResponse = schema.Infer<typeof TokenResponse>;
