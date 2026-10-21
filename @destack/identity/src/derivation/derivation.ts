import type { JWK } from "jose";
import { Label } from "../label/label.ts";

/** The bits of a derived secret: 256, the HKDF-SHA256 output length. */
const SECRET_BITS = 256;

/** The bits a private key's scalar reduces from: 320, 64 beyond the curve's order (FIPS 186-5). */
const SCALAR_BITS = 320;

/** The curve of derived private keys. */
const P256 = { name: "ECDSA", namedCurve: "P-256" } as const;

/** The order of P-256's base point. */
const P256_ORDER = 0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551n;

/** The DER of a PKCS #8 P-256 private key up to its 32-byte scalar, carrying no public point. */
const PKCS8_PREFIX = "3041020100301306072a8648ce3d020106082a8648ce3d030107042730250201010420";

/** What derives secrets and private keys under labels. */
export interface Deriver {
    /** Derive a 256-bit secret under a label. */
    readonly derive: (label: Label) => Promise<Uint8Array<ArrayBuffer>>;
    /** Derive a P-256 private key under a label, as a JSON Web Key. */
    readonly derivePrivateKey: (label: Label) => Promise<JWK>;
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
    async secret(root: CryptoKey, label: Label): Promise<Uint8Array<ArrayBuffer>> {
        const bits = await crypto.subtle.deriveBits(hkdf(label, SECRET_BITS), root, SECRET_BITS);

        return new Uint8Array(bits);
    },

    /** Derive a P-256 private key under a label. */
    async privateKey(root: CryptoKey, label: Label): Promise<JWK> {
        // reduce the derived bits to a scalar below the curve's order
        const bits = await crypto.subtle.deriveBits(hkdf(label, SCALAR_BITS), root, SCALAR_BITS);
        const scalar = (BigInt(`0x${new Uint8Array(bits).toHex()}`) % (P256_ORDER - 1n)) + 1n;

        // import the scalar as a PKCS #8 key without its public point
        const der = Uint8Array.fromHex(`${PKCS8_PREFIX}${scalar.toString(16).padStart(64, "0")}`);
        const key = await crypto.subtle.importKey("pkcs8", der, P256, true, ["sign"]);

        return crypto.subtle.exportKey("jwk", key);
    },

    /** Derive a non-extractable AES-256-GCM key under a label, such as a keyring's key. */
    encryptionKey(root: CryptoKey, label: Label): Promise<CryptoKey> {
        const algorithm = { name: "AES-GCM", length: SECRET_BITS } as const;

        return crypto.subtle.deriveKey(hkdf(label, SECRET_BITS), root, algorithm, false, [
            "encrypt",
            "decrypt",
        ]);
    },

    /** Import a secret's bytes as an HKDF root key that derives and never leaves. */
    root(bytes: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
        return crypto.subtle.importKey("raw", bytes, "HKDF", false, ["deriveBits", "deriveKey"]);
    },
};

/** Name an HKDF-SHA256 derivation under a label for an output length, with no salt. */
function hkdf(label: Label, bits: number) {
    return {
        name: "HKDF" as const,
        hash: "SHA-256",
        salt: new Uint8Array(),
        info: Label.encodeExpansion(label, bits / 8),
    };
}
