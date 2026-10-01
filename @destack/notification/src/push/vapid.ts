import { SignJWT } from "jose";

/** How long a token stays valid, in seconds: half the day RFC 8292 allows. */
const TOKEN_SECONDS = 12 * 60 * 60;

/** A push sender's VAPID identity, as RFC 8292 defines it. */
export class Vapid {
    /** The signing key pair. */
    readonly keys: CryptoKeyPair;
    /** The contact, a mailto: or https: URL. */
    readonly subject: string;

    /** Keep the sender's keys and contact. */
    constructor(keys: CryptoKeyPair, subject: string) {
        if (!/^(?:mailto:|https:)/.test(subject)) {
            throw new TypeError("a push sender's contact is a mailto: or https: URL");
        }
        this.keys = keys;
        this.subject = subject;
    }

    /** Read the public key browsers subscribe with, base64url encoded. */
    async publicKey(): Promise<string> {
        const raw = new Uint8Array(await crypto.subtle.exportKey("raw", this.keys.publicKey));

        return raw.toBase64({ alphabet: "base64url", omitPadding: true });
    }

    /** Sign the Authorization header for one push resource. */
    async authorization(url: string, now: number): Promise<string> {
        // claim the push service's origin until the token lapses
        const token = await new SignJWT({ sub: this.subject })
            .setProtectedHeader({ typ: "JWT", alg: "ES256" })
            .setAudience(new URL(url).origin)
            .setExpirationTime(Math.floor(now / 1000) + TOKEN_SECONDS)
            .sign(this.keys.privateKey);

        return `vapid t=${token}, k=${await this.publicKey()}`;
    }
}
