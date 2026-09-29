import { constants, createHash, createPublicKey, generateKeyPairSync, sign } from "node:crypto";
import { Key, Signature } from "@tufjs/models";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import type { ReleaseKeys, RenewalKeys } from "../repository/repository.ts";

/** An offline or automated TUF signing key. */
export class SigningKey {
    /** Public key included in trusted root metadata. */
    readonly public: Key;
    /** PKCS#8 private key retained outside the repository. */
    readonly private: string;

    /** Read a PEM key and derive its public identity. */
    constructor(privateKey: string) {
        this.private = privateKey;
        this.public = readPublicKey(
            createPublicKey(privateKey).export({ format: "pem", type: "spki" }).toString(),
        );
    }

    /** Generate an independent key for one metadata role. */
    static generate(): SigningKey {
        const pair = generateKeyPairSync("ed25519");

        return new SigningKey(pair.privateKey.export({ format: "pem", type: "pkcs8" }).toString());
    }

    /** Sign the canonical bytes supplied by the TUF metadata implementation. */
    sign(bytes: Buffer): Signature {
        // select the standard signature scheme declared by the public key
        const isRsa = this.public.keyType === "rsa";
        const signature = isRsa
            ? sign("sha256", bytes, {
                  key: this.private,
                  padding: constants.RSA_PKCS1_PSS_PADDING,
                  saltLength: 32,
              })
            : sign(null, bytes, this.private);

        return new Signature({
            keyID: this.public.keyID,
            sig: signature.toString("hex"),
        });
    }
}

/** Read a supported public key without needing access to its private key. */
export function readPublicKey(pem: string): Key {
    const key = createPublicKey(pem);

    // retain the compact Ed25519 representation used by routine release signing
    if (key.asymmetricKeyType === "ed25519") {
        const jwk = key.export({ format: "jwk" });
        if (!jwk.x) {
            throw new Error("missing Ed25519 public key");
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
            throw new Error("RSA signing keys must contain at least 2048 bits");
        }
        const bytes = key.export({ format: "der", type: "spki" });

        return new Key({
            keyID: createHash("sha256").update(bytes).digest("hex"),
            keyType: "rsa",
            scheme: "rsassa-pss-sha256",
            keyVal: { public: key.export({ format: "pem", type: "spki" }).toString() },
        });
    } else {
        throw new Error("unsupported signing key algorithm");
    }
}

/** Read separate online role keys from the selected private directory. */
export async function readReleaseKeys(): Promise<ReleaseKeys> {
    // load the additional authorization key only for publication
    const renewal = await readRenewalKeys();
    const directory = process.env.DESTACK_RELEASE_KEYS!;
    const targets = new SigningKey(await readFile(join(directory, "targets.pem"), "utf8"));

    return { targets, ...renewal };
}

/** Read freshness keys without requesting the targets signing credential. */
export async function readRenewalKeys(): Promise<RenewalKeys> {
    // load only the two roles permitted to maintain freshness
    const directory = process.env.DESTACK_RELEASE_KEYS;
    if (!directory) {
        throw new Error("set DESTACK_RELEASE_KEYS to the private online key directory");
    }
    const keys = await Promise.all(
        ["snapshot", "timestamp"].map(
            async (role) => new SigningKey(await readFile(join(directory, `${role}.pem`), "utf8")),
        ),
    );

    return { snapshot: keys[0]!, timestamp: keys[1]! };
}
