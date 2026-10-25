import type { Crypto, Random } from "@destack/environment";

/** The random bytes of a code verifier, 256 bits in 43 base64url characters (RFC 7636 4.1). */
const VERIFIER_BYTES = 32;

/** Proof Key for Code Exchange with the S256 method (RFC 7636): verifiers and their challenges. */
export const Pkce = {
    /** Draw a new code verifier. */
    verifier(random: Random): string {
        return random.bytes(VERIFIER_BYTES).toBase64({ alphabet: "base64url", omitPadding: true });
    },

    /** Derive a verifier's S256 challenge: its SHA-256 in unpadded base64url (RFC 7636 4.2). */
    async challenge(verifier: string, crypto: Crypto): Promise<string> {
        const digest = await crypto.digest("SHA-256", new TextEncoder().encode(verifier));

        return new Uint8Array(digest).toBase64({ alphabet: "base64url", omitPadding: true });
    },
};
