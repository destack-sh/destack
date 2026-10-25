import type { Crypto } from "@destack/environment";
import type { Deriver } from "@destack/identity";

/** Read the deployment's key that sensitive call inputs are fingerprinted under, on a WebCrypto. */
export type CallKey = (crypto: Crypto) => Promise<CryptoKey>;

/** The label the call key derives under. */
const CALL_KEY_LABEL = { label: "@destack/object/call-key", context: "" } as const;

/** Import and derive the keys sensitive call inputs are fingerprinted under. */
export const CallKey = {
    /** Import a deployment's key from 32 secret bytes, on an environment's WebCrypto. */
    import(bytes: Uint8Array<ArrayBuffer>, crypto: Crypto): Promise<CryptoKey> {
        return crypto.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
    },

    /** Derive the call key from a keyring or an installation's root. */
    async of(keyring: Pick<Deriver, "derive">, crypto: Crypto): Promise<CryptoKey> {
        return CallKey.import(await keyring.derive(CALL_KEY_LABEL), crypto);
    },
};
