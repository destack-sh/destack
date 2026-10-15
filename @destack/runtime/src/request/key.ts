/** Read the deployment's key that sensitive call inputs are fingerprinted under. */
export type CallKey = () => Promise<CryptoKey>;

/** The label the call key derives under. */
const CALL_KEY_LABEL = "destack call key v1";

/** Import and derive the keys sensitive call inputs are fingerprinted under. */
export const CallKey = {
    /** Import a deployment's key from 32 secret bytes. */
    import(bytes: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
        return crypto.subtle.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [
            "sign",
        ]);
    },

    /** Derive the call key from a keyring or an installation's root. */
    async of(keyring: {
        readonly derive: (label: string) => Promise<Uint8Array<ArrayBuffer>>;
    }): Promise<CryptoKey> {
        return CallKey.import(await keyring.derive(CALL_KEY_LABEL));
    },
};
