import { and, eq } from "@destack/db";
import type { Call } from "@destack/object";
import { oauthRefreshToken, oauthAccessToken } from "../../stack/authentication/oauth/token.ts";
import * as base from "../../object/oauth.ts";
import type { OAuthConsent } from "../../object/oauth.ts";

/** The random bytes of a client identifier, 192 bits in 32 base64url characters. */
const CLIENT_ID_BYTES = 24;

/** One call of a client method. */
type ClientCall = Call<typeof base.oauthClient.table>;

/** One call of a consent method. */
type ConsentCall = Call<typeof base.oauthConsent.table>;

/** Clients on the server with a public identifier the provider mints. */
export const oauthClient = base.oauthClient.handle({ create: register });

/** Consents on the server; revoking one ends the client's tokens for the user. */
export const oauthConsent = base.oauthConsent.handle({ revoke });

/** Register a client under a new random public identifier. */
async function register(call: ClientCall, next: (call?: ClientCall) => Promise<unknown>) {
    const bytes = crypto.getRandomValues(new Uint8Array(CLIENT_ID_BYTES));
    const clientId = bytes.toBase64({ alphabet: "base64url", omitPadding: true });

    return next(call.with({ input: { ...call.input, clientId } }));
}

/** Revoke a consent with its client's tokens for the user. */
async function revoke(call: ConsentCall, next: (call?: ConsentCall) => Promise<unknown>) {
    // end the client's access and refresh tokens of the user
    const consent = call.target as OAuthConsent;
    for (const table of [oauthAccessToken, oauthRefreshToken]) {
        await call.database
            .delete(table)
            .where(and(eq(table.clientId, consent.clientId), eq(table.userId, consent.userId)));
    }

    return next();
}
