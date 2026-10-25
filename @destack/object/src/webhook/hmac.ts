import type { Crypto } from "@destack/environment";

/** An HMAC-SHA256 key that signs and verifies webhook content. */
export class HmacKey {
    /** The imported key. */
    readonly #key: CryptoKey;
    /** The WebCrypto the key signs and verifies on. */
    readonly #crypto: Crypto;

    /** Wrap an imported key with its WebCrypto. */
    private constructor(key: CryptoKey, crypto: Crypto) {
        this.#key = key;
        this.#crypto = crypto;
    }

    /** Import raw bytes as a key that signs and verifies, on an environment's WebCrypto. */
    static async of(bytes: Uint8Array<ArrayBuffer>, crypto: Crypto): Promise<HmacKey> {
        const key = await crypto.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [
            "sign",
            "verify",
        ]);

        return new HmacKey(key, crypto);
    }

    /** Sign content, answering its digest. */
    async sign(content: Uint8Array<ArrayBuffer>): Promise<Uint8Array<ArrayBuffer>> {
        return new Uint8Array(await this.#crypto.sign("HMAC", this.#key, content));
    }

    /** Check that a digest signs content. */
    verify(digest: Uint8Array<ArrayBuffer>, content: Uint8Array<ArrayBuffer>): Promise<boolean> {
        return this.#crypto.verify("HMAC", this.#key, digest, content);
    }
}
