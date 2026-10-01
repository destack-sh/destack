/** Read the deployment's key that sensitive call inputs are fingerprinted under, the same on every instance. */
export type CallKey = () => Promise<CryptoKey>;

/** Import and derive the keys sensitive call inputs are fingerprinted under. */
export const CallKey = {
    /** Import a deployment's key from 32 secret bytes. */
    import(bytes: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
        return crypto.subtle.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [
            "sign",
        ]);
    },

    /** Derive the key of one installation from the host's key, so no installation reads another's. */
    async derive(key: CryptoKey, installation: string): Promise<Uint8Array<ArrayBuffer>> {
        const label = new TextEncoder().encode(`call:${installation}`);

        return new Uint8Array(await crypto.subtle.sign("HMAC", key, label));
    },
};
