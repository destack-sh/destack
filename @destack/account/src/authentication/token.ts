import { schema } from "@destack/schema";

/** The milliseconds in a second, as OAuth counts expiries in seconds. */
const SECOND_MILLISECONDS = 1000;

/** A token endpoint's answer (RFC 6749 5.1), with the provider's absolute expiry. */
export const TokenResponse = Object.assign(
    schema.looseObject({
        /** The access token. */
        access_token: schema.string().min(1),
        /** When the access token expires, in UTC epoch seconds. */
        expires_at: schema.number().int(),
        /** The refresh token, granted for offline access. */
        refresh_token: schema.string().min(1).exactOptional(),
        /** The ID token naming the signed-in person, granted with the openid scope. */
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
    },
);
/** A token endpoint's answer. */
export type TokenResponse = schema.Infer<typeof TokenResponse>;
