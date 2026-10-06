import type { JWK } from "jose";

/** The bits of a derived secret: 256, the HKDF-SHA256 output length. */
const SECRET_BITS = 256;

/** The bits a private key's scalar reduces from: 320, 64 beyond the curve's order as FIPS 186-5 asks. */
const SCALAR_BITS = 320;

/** The curve of derived private keys. */
const P256 = { name: "ECDSA", namedCurve: "P-256" } as const;

/** The order of P-256's base point. */
const P256_ORDER = 0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551n;

/** The DER of a PKCS #8 P-256 private key up to its 32-byte scalar, carrying no public point. */
const PKCS8_PREFIX = "3041020100301306072a8648ce3d020106082a8648ce3d030107042730250201010420";

/** What derives secrets and private keys under labels, the same for a label until its root changes. */
export interface Deriver {
    /** Derive a 256-bit secret under a label. */
    derive(label: string): Promise<Uint8Array<ArrayBuffer>>;
    /** Derive a P-256 private key under a label, as a JSON Web Key. */
    derivePrivateKey(label: string): Promise<JWK>;
}

/** Derive under labels from an HKDF root key, as a key management service derives its keys. */
export const Derivation = {
    /** Derive the secrets and private keys of an HKDF root key. */
    of(root: CryptoKey): Deriver {
        return {
            derive: (label) => Derivation.secret(root, label),
            derivePrivateKey: (label) => Derivation.privateKey(root, label),
        };
    },

    /** Derive a 256-bit secret under a label. */
    async secret(root: CryptoKey, label: string): Promise<Uint8Array<ArrayBuffer>> {
        return new Uint8Array(await crypto.subtle.deriveBits(hkdf(label), root, SECRET_BITS));
    },

    /** Derive a P-256 private key under a label, reducing 320 derived bits into a scalar as FIPS 186-5 does. */
    async privateKey(root: CryptoKey, label: string): Promise<JWK> {
        // reduce the derived bits to a scalar below the curve's order
        const bits = await crypto.subtle.deriveBits(hkdf(label), root, SCALAR_BITS);
        const scalar = (BigInt(`0x${new Uint8Array(bits).toHex()}`) % (P256_ORDER - 1n)) + 1n;

        // import the scalar as a PKCS #8 key without its public point
        const der = Uint8Array.fromHex(`${PKCS8_PREFIX}${scalar.toString(16).padStart(64, "0")}`);
        const key = await crypto.subtle.importKey("pkcs8", der, P256, true, ["sign"]);

        return crypto.subtle.exportKey("jwk", key);
    },

    /** Derive a non-extractable AES-256-GCM key under a label, such as the key a keyring encrypts under. */
    encryptionKey(root: CryptoKey, label: string): Promise<CryptoKey> {
        return crypto.subtle.deriveKey(hkdf(label), root, { name: "AES-GCM", length: 256 }, false, [
            "encrypt",
            "decrypt",
        ]);
    },

    /** Import a secret's bytes as an HKDF root key that derives and never leaves. */
    root(bytes: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
        return crypto.subtle.importKey("raw", bytes, "HKDF", false, ["deriveBits", "deriveKey"]);
    },
};

/** Name an HKDF-SHA256 derivation of a root key under a label, with no salt. */
function hkdf(label: string) {
    return {
        name: "HKDF" as const,
        hash: "SHA-256",
        salt: new Uint8Array(),
        info: new TextEncoder().encode(label),
    };
}
