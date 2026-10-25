import type { Crypto } from "@destack/environment";
import { type Deriver, Jwt, KeyPair, PublicKey } from "@destack/identity";

/** How long a token stays valid, in seconds: half the day RFC 8292 allows. */
const TOKEN_SECONDS = 12 * 60 * 60;

/** The label of the installation's VAPID key, distinct from every other key its keys derive. */
const LABEL = { label: "@destack/message/vapid", context: "" } as const;

/** A push sender's VAPID identity, as RFC 8292 defines it. */
export class Vapid {
    /** The signing key pair. */
    readonly keys: KeyPair;
    /** The contact, a mailto: or https: URL. */
    readonly subject: string;
    /** The WebCrypto the keys sign and export with. */
    readonly crypto: Crypto;

    /** Create a sender from its keys and contact, refusing a contact push services cannot reach. */
    constructor(keys: KeyPair, subject: string, crypto: Crypto) {
        if (!/^(?:mailto:|https:)/u.test(subject)) {
            throw new TypeError(`push sender contact ${subject} is not a mailto: or https: URL`);
        }
        this.keys = keys;
        this.subject = subject;
        this.crypto = crypto;
    }

    /** Derive the installation's VAPID keys from its keys, the same for every derivation. */
    static async derive(
        root: Pick<Deriver, "derivePrivateKey">,
        subject: string,
        crypto: Crypto,
    ): Promise<Vapid> {
        const jwk = await root.derivePrivateKey(LABEL);

        return new Vapid(await KeyPair.import(jwk, crypto), subject, crypto);
    }

    /** Read the public key browsers subscribe with, base64url encoded. */
    publicKey(): string {
        return PublicKey.raw(this.keys.publicKey).toBase64({
            alphabet: "base64url",
            omitPadding: true,
        });
    }

    /** Sign the Authorization header for one push resource. */
    async authorization(url: string, now: number): Promise<string> {
        // claim the push service's origin until the token lapses
        const claims = {
            sub: this.subject,
            aud: new URL(url).origin,
            exp: Math.floor(now / 1000) + TOKEN_SECONDS,
        };
        const header = { typ: "JWT", alg: "ES256" } as const;
        const token = await Jwt.sign(claims, header, this.keys.privateKey, this.crypto);

        return `vapid t=${token}, k=${this.publicKey()}`;
    }
}
