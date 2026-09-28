/** How long a token stays valid, in seconds: half the day RFC 8292 allows. */
const TOKEN_SECONDS = 12 * 60 * 60;

/** The signature algorithm RFC 8292 fixes: ECDSA over P-256 with SHA-256. */
const SIGNATURE = { name: "ECDSA", hash: "SHA-256" } as const;

/** A push sender's VAPID identity, as RFC 8292 defines it. */
export class Vapid {
    /** The signing key pair. */
    readonly keys: CryptoKeyPair;
    /** The contact, a mailto: or https: URL. */
    readonly subject: string;

    /** Hold the sender's keys and contact. */
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
        // claim the push service's origin
        const header = encode({ typ: "JWT", alg: "ES256" });
        const claims = encode({
            aud: new URL(url).origin,
            exp: Math.floor(now / 1000) + TOKEN_SECONDS,
            sub: this.subject,
        });

        // sign the header and claims
        const signed = `${header}.${claims}`;
        const signature = new Uint8Array(
            await crypto.subtle.sign(
                SIGNATURE,
                this.keys.privateKey,
                new TextEncoder().encode(signed),
            ),
        );
        const token = `${signed}.${signature.toBase64({ alphabet: "base64url", omitPadding: true })}`;

        return `vapid t=${token}, k=${await this.publicKey()}`;
    }
}

/** Encode a JSON value as base64url text. */
function encode(value: unknown): string {
    return new TextEncoder()
        .encode(JSON.stringify(value))
        .toBase64({ alphabet: "base64url", omitPadding: true });
}
