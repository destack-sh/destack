import { createHash, createPublicKey } from "node:crypto";
import { canonicalize } from "@tufjs/canonical-json";
import {
    Key,
    type Metadata,
    type Root,
    Signature,
    type Snapshot,
    type Targets,
    type Timestamp,
} from "@tufjs/models";
import type { Crypto } from "@destack/environment";
import { UpdateError } from "../error/index.ts";

/** The PEM label of a PKCS#8 private key. */
const PRIVATE_KEY_LABEL = "PRIVATE KEY";

/** The base64 characters on each PEM line, as RFC 7468 sets. */
const PEM_LINE_LENGTH = 64;

/** The WebCrypto algorithms of each TUF signature scheme, to generate, import and sign with. */
const SCHEMES = {
    ed25519: {
        generate: { name: "Ed25519" },
        import: { name: "Ed25519" },
        sign: { name: "Ed25519" },
    },
    "rsassa-pss-sha256": {
        // the 2048-bit modulus TUF requires at least, with the F4 exponent
        generate: {
            name: "RSA-PSS",
            hash: "SHA-256",
            modulusLength: 2048,
            publicExponent: new Uint8Array([1, 0, 1]),
        },
        import: { name: "RSA-PSS", hash: "SHA-256" },
        // the SHA-256 digest's 32 bytes of salt
        sign: { name: "RSA-PSS", saltLength: 32 },
    },
} as const;

/** A TUF signature scheme a signing key uses. */
export type SigningScheme = keyof typeof SCHEMES;

/** An offline or automated TUF signing key. */
export class SigningKey {
    /** The public key included in trusted root metadata. */
    readonly public: Key;
    /** The PKCS#8 PEM private key, kept outside the repository. */
    readonly private: string;
    /** The private key WebCrypto signs with. */
    readonly #key: CryptoKey;
    /** The WebCrypto operations the key signs through. */
    readonly #crypto: Crypto;

    /** Create a key from its PEM, its imported private key and its public identity. */
    private constructor(privateKey: string, key: CryptoKey, publicKey: Key, crypto: Crypto) {
        // set the private key, its imported form and its public identity
        this.private = privateKey;
        this.#key = key;
        this.public = publicKey;
        this.#crypto = crypto;
    }

    /** Import a PKCS#8 PEM key and derive its public identity. */
    static async import(privateKey: string, crypto: Crypto): Promise<SigningKey> {
        // derive the public identity and the scheme it declares
        const publicKey = readPublicKey(
            createPublicKey(privateKey).export({ format: "pem", type: "spki" }),
        );
        const scheme = SCHEMES[schemeOf(publicKey)];

        // import the private key for signing only
        const der = Buffer.from(privateKey.replace(/-----[A-Z ]+-----|\s/gu, ""), "base64");
        const key = await crypto.importKey("pkcs8", der, scheme.import, false, ["sign"]);

        return new SigningKey(privateKey, key, publicKey, crypto);
    }

    /** Generate an independent key in a scheme for one metadata role. */
    static async generate(scheme: SigningScheme, crypto: Crypto): Promise<SigningKey> {
        // generate an extractable pair, then import its private key as a PEM document
        const pair = await crypto.generateKey(SCHEMES[scheme].generate, true, ["sign", "verify"]);
        if (!("privateKey" in pair)) {
            throw new TypeError(`${scheme} generation returned no key pair`);
        }
        const der = await crypto.exportKey("pkcs8", pair.privateKey);

        return await SigningKey.import(pem(new Uint8Array(der)), crypto);
    }

    /** Sign metadata's canonical bytes, adding the signature under this key's id. */
    async sign(metadata: Metadata<Root | Timestamp | Snapshot | Targets>): Promise<void> {
        // sign the canonical JSON TUF verifies
        const bytes = new TextEncoder().encode(canonicalize(metadata.signed.toJSON()));
        const algorithm = SCHEMES[schemeOf(this.public)].sign;
        const signature = await this.#crypto.sign(algorithm, this.#key, bytes);

        // add the signature under the key's id
        metadata.signatures[this.public.keyID] = new Signature({
            keyID: this.public.keyID,
            sig: Buffer.from(signature).toString("hex"),
        });
    }
}

/** Read the signature scheme a public key declares. */
function schemeOf(key: Key): SigningScheme {
    if (key.scheme !== "ed25519" && key.scheme !== "rsassa-pss-sha256") {
        throw UpdateError.of("INVALID_SIGNING_KEY", `unsupported signing scheme ${key.scheme}`);
    }

    return key.scheme;
}

/** Encode DER bytes as a PKCS#8 PEM document. */
function pem(der: Uint8Array): string {
    const encoded = Buffer.from(der).toString("base64");
    const lines = Array.from({ length: Math.ceil(encoded.length / PEM_LINE_LENGTH) }, (_, index) =>
        encoded.slice(index * PEM_LINE_LENGTH, (index + 1) * PEM_LINE_LENGTH),
    );

    return `-----BEGIN ${PRIVATE_KEY_LABEL}-----\n${lines.join("\n")}\n-----END ${PRIVATE_KEY_LABEL}-----\n`;
}

/** Read a supported public key without needing access to its private key. */
export function readPublicKey(document: string): Key {
    const key = createPublicKey(document);

    // retain the compact Ed25519 representation used by routine release signing
    if (key.asymmetricKeyType === "ed25519") {
        const jwk = key.export({ format: "jwk" });
        if (jwk.x === undefined || jwk.x === "") {
            throw UpdateError.of("INVALID_SIGNING_KEY", "missing Ed25519 public key");
        }
        const bytes = Buffer.from(jwk.x, "base64url");

        return new Key({
            keyID: createHash("sha256").update(bytes).digest("hex"),
            keyType: "ed25519",
            scheme: "ed25519",
            keyVal: { public: bytes.toString("hex") },
        });
    }
    // use RSA-PSS for the existing hardware root keys
    else if (key.asymmetricKeyType === "rsa") {
        const modulusLength = key.asymmetricKeyDetails?.modulusLength;
        if (modulusLength === undefined || modulusLength < 2048) {
            throw UpdateError.of(
                "INVALID_SIGNING_KEY",
                "RSA signing keys must contain at least 2048 bits",
            );
        }
        const bytes = key.export({ format: "der", type: "spki" });

        return new Key({
            keyID: createHash("sha256").update(bytes).digest("hex"),
            keyType: "rsa",
            scheme: "rsassa-pss-sha256",
            keyVal: { public: key.export({ format: "pem", type: "spki" }) },
        });
    } else {
        throw UpdateError.of("INVALID_SIGNING_KEY", "unsupported signing key algorithm");
    }
}
