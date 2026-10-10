import { Derivation, type Deriver } from "@destack/identity";
/** Read the deployment's key that sensitive call inputs are fingerprinted under, the same on every instance. */
export type CallKey = () => Promise<CryptoKey>;

/** The label a deployment's call key derives from its keyring under. */
const CALL_KEY_LABEL = "destack call key v1";

/** The label an installation's root secret derives from its call key under. */
const ROOT_LABEL = "destack installation root v1";

/** Import and derive the keys sensitive call inputs are fingerprinted under. */
export const CallKey = {
    /** Import a deployment's key from 32 secret bytes. */
    import(bytes: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
        return crypto.subtle.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [
            "sign",
        ]);
    },

    /** Derive a deployment's key from its keyring, the same on every instance holding its root key. */
    async of(keyring: {
        derive(label: string): Promise<Uint8Array<ArrayBuffer>>;
    }): Promise<CryptoKey> {
        return CallKey.import(await keyring.derive(CALL_KEY_LABEL));
    },

    /** Derive the key of one installation from the host's key, so no installation reads another's. */
    async derive(key: CryptoKey, installation: string): Promise<Uint8Array<ArrayBuffer>> {
        const label = new TextEncoder().encode(`call:${installation}`);

        return new Uint8Array(await crypto.subtle.sign("HMAC", key, label));
    },

    /** Derive an installation's own secrets from its call key, through an HKDF root under a label of their own. */
    async deriver(key: CryptoKey): Promise<Deriver> {
        const label = new TextEncoder().encode(ROOT_LABEL);
        const root = new Uint8Array(await crypto.subtle.sign("HMAC", key, label));

        return Derivation.of(await Derivation.root(root));
    },
};
