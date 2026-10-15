import type { CallOf, ResultOf } from "@destack/object";
import { Revocation } from "../credential/revocation.ts";
import { oauthConsent } from "../object/oauth-consent.ts";

/** Implement OAuth consents. */
export function implementOAuthConsent(): typeof oauthConsent {
    return oauthConsent.implement({ methods: { revoke } });
}

/** Revoke a consent, which ends the use of the tokens issued under it until granted again. */
async function revoke(
    call: CallOf<typeof oauthConsent, "revoke">,
): Promise<ResultOf<typeof oauthConsent, "revoke">> {
    // refuse a consent revoked before
    Revocation.requireUnrevoked(call.target, "consent");

    return call.update({ revokedAt: call.now });
}
