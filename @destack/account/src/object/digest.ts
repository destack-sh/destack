/** SHA-256 digests of text, as secrets, codes, PKCE verifiers and key thumbprints keep them. */
export class Digest {
    /** Hash text with SHA-256 into lowercase hex. */
    static async hex(text: string): Promise<string> {
        return (await Digest.#bytes(text)).toHex();
    }

    /** Hash text with SHA-256 into unpadded base64url, as PKCE S256 and JWK thumbprints encode it. */
    static async base64url(text: string): Promise<string> {
        return (await Digest.#bytes(text)).toBase64({ alphabet: "base64url", omitPadding: true });
    }

    /** Hash the UTF-8 bytes of text with SHA-256. */
    static async #bytes(text: string): Promise<Uint8Array> {
        return new Uint8Array(
            await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)),
        );
    }
}
