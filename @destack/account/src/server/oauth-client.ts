import type { CallOf, NextOf, ResultOf } from "@destack/object";
import { oauthClient } from "../object/oauth-client.ts";
import { OAuthConsent } from "../object/oauth-consent.ts";

/** The random bytes of a client identifier, 192 bits in 32 base64url characters. */
const CLIENT_ID_BYTES = 24;

/** Implement OAuth clients. */
export function implementOAuthClient(): typeof oauthClient {
    return oauthClient.implement({
        methods: {
            create,
            // end the consents and tokens the provider granted it with the client
            delete: async (call, next) => {
                await OAuthConsent.withdraw(call, call.target.clientId);

                return next();
            },
        },
    });
}

/** Register a client under a new random public identifier. */
async function create(
    call: CallOf<typeof oauthClient, "create">,
    next: NextOf<typeof oauthClient, "create">,
): Promise<ResultOf<typeof oauthClient, "create">> {
    // draw the public identifier
    const bytes = crypto.getRandomValues(new Uint8Array(CLIENT_ID_BYTES));
    const clientId = bytes.toBase64({ alphabet: "base64url", omitPadding: true });

    return next(call.with({ input: { ...call.input, clientId } }));
}
