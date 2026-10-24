import type { Crypto, Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { Base64Url } from "./base64.ts";

/** The bytes of a content key and of a key-encrypting key: 256 bits for A256GCM and A256KW. */
const KEY_BYTES = 32;

/** The bytes of an AES-GCM initialization vector, 96 bits (RFC 7518 5.3, 4.7.1). */
const IV_BYTES = 12;

/** The bytes of an AES-GCM authentication tag, 128 bits. */
const TAG_BYTES = 16;

/** The content encryption of every JWE (RFC 7518 5.3). */
const CONTENT = { name: "AES-GCM", length: KEY_BYTES * 8 } as const;

/** The curve of ECDH-ES keys. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;

/** The bytes of a P-256 ECDH shared secret, its x coordinate (RFC 7518 4.6.2). */
const SHARED_SECRET_BYTES = 32;

/** The header members a JWE refuses: extensions, compression and the KDF's party data. */
const REFUSED_MEMBERS = ["crit", "zip", "apu", "apv"] as const;

/** The key management each WebCrypto key algorithm opens. */
const MANAGEMENT: ReadonlyMap<string, KeyManagement> = new Map([
    ["AES-GCM", "A256GCMKW"],
    ["ECDH", "ECDH-ES+A256KW"],
]);

/** How a JWE's content key reaches its recipient (RFC 7518 4.6, 4.7). */
export type KeyManagement = "A256GCMKW" | "ECDH-ES+A256KW";

/** A JWE protected header (RFC 7516 4.1). */
export const JweHeader = schema.looseObject({
    /** How the content key reaches its recipient. */
    alg: schema.enum(["A256GCMKW", "ECDH-ES+A256KW"]),
    /** The content encryption. */
    enc: schema.literal("A256GCM"),
    /** The identifier of the recipient's key. */
    kid: schema.string().min(1).exactOptional(),
});
/** A JWE protected header. */
export type JweHeader = schema.Infer<typeof JweHeader>;

/** The ephemeral public key of an ECDH-ES header (RFC 7518 4.6.1.1). */
const EphemeralKey = schema.looseObject({
    kty: schema.literal("EC"),
    crv: schema.literal("P-256"),
    x: schema.string().min(1),
    y: schema.string().min(1),
});
/** The ephemeral public key of an ECDH-ES header. */
type EphemeralKey = schema.Infer<typeof EphemeralKey>;

/** The members of an A256GCMKW header: its key wrap's vector and tag (RFC 7518 4.7.1). */
const WrapMembers = schema.looseObject({
    iv: schema.string().min(1),
    tag: schema.string().min(1),
});

/** What a decrypted JWE has: its protected header and its plaintext. */
export interface DecryptedJwe {
    /** The protected header. */
    readonly header: JweHeader;
    /** The plaintext. */
    readonly plaintext: Uint8Array<ArrayBuffer>;
}

/** Encrypted bytes in the compact serialization, as A256GCM under a wrapped content key. */
export const Jwe = {
    /** Encrypt bytes to a key: an AES-GCM key for A256GCMKW, an ECDH public key for ECDH-ES. */
    async encrypt(
        plaintext: Uint8Array<ArrayBuffer>,
        header: JweHeader,
        key: CryptoKey,
        environment: Pick<Environment, "crypto" | "random">,
    ): Promise<string> {
        // wrap a fresh content key for the recipient, its members joining the header
        const { crypto, random } = environment;
        const content = random.bytes(KEY_BYTES);
        const wrapped =
            header.alg === "A256GCMKW"
                ? await wrapWithKey(content, key, environment)
                : await wrapWithAgreement(content, key, environment);
        const encodedHeader = Base64Url.encode(JSON.stringify({ ...header, ...wrapped.members }));

        // encrypt the content under the header as additional data, splitting off the tag
        const iv = random.bytes(IV_BYTES);
        const contentKey = await crypto.importKey("raw", content, CONTENT, false, ["encrypt"]);
        const sealed = new Uint8Array(
            await crypto.encrypt(
                { name: "AES-GCM", iv, additionalData: new TextEncoder().encode(encodedHeader) },
                contentKey,
                plaintext,
            ),
        );
        const body = sealed.subarray(0, sealed.length - TAG_BYTES);
        const tag = sealed.subarray(sealed.length - TAG_BYTES);

        return [encodedHeader, wrapped.key, iv, body, tag]
            .map((segment) => (typeof segment === "string" ? segment : Base64Url.encode(segment)))
            .join(".");
    },

    /** Decrypt a JWE with its key: the AES-GCM key, or the ECDH private key it was sealed to. */
    async decrypt(compact: string, key: CryptoKey, crypto: Crypto): Promise<DecryptedJwe> {
        // read the five segments and the header, refusing members it cannot honor
        const [encodedHeader, encodedKey, encodedIv, encodedBody, encodedTag] = segments(compact);
        const members = Base64Url.object(encodedHeader);
        const refused = REFUSED_MEMBERS.find((member) => member in members);
        if (refused !== undefined) {
            throw IdentityError.of("CIPHERTEXT_MALFORMED", `the ciphertext header has ${refused}`);
        }
        const parsed = JweHeader.safeParse(members);
        if (!parsed.success) {
            throw IdentityError.of("CIPHERTEXT_MALFORMED", "the ciphertext has an invalid header");
        }
        const header = parsed.data;

        // refuse a key of another key management than the header's
        const management = MANAGEMENT.get(key.algorithm.name);
        if (management !== header.alg) {
            throw IdentityError.of(
                "DECRYPTION_FAILED",
                `the ciphertext is sealed by ${header.alg}, the key opens ${management ?? "none"}`,
            );
        }

        // decode the wrapped key, the vector, the body and the tag, refusing other lengths
        const wrapped = Base64Url.decode(encodedKey);
        const iv = sized(Base64Url.decode(encodedIv), IV_BYTES, "vector");
        const sealed = concat([
            Base64Url.decode(encodedBody),
            sized(Base64Url.decode(encodedTag), TAG_BYTES, "tag"),
        ]);
        const unwrap =
            header.alg === "A256GCMKW"
                ? readWrap(members)
                : { epk: readMember(EphemeralKey, members["epk"], "ephemeral key") };

        // unwrap the content key and decrypt, refusing any failed authentication alike
        try {
            const content =
                "epk" in unwrap
                    ? await unwrapWithAgreement(wrapped, unwrap.epk, key, crypto)
                    : await unwrapWithKey(wrapped, unwrap, key, crypto);
            const additionalData = new TextEncoder().encode(encodedHeader);
            const plaintext = await crypto.decrypt(
                { name: "AES-GCM", iv, additionalData },
                content,
                sealed,
            );

            return { header, plaintext: new Uint8Array(plaintext) };
        } catch (error) {
            throw IdentityError.of("DECRYPTION_FAILED", "ciphertext authentication failed", {
                cause: error,
            });
        }
    },

    /** Read a JWE's protected header without decrypting it. */
    header(compact: string): Record<string, unknown> {
        const [encodedHeader] = segments(compact);

        return Base64Url.object(encodedHeader);
    },
};

/** Wrap a content key with an AES-GCM key, its vector and tag in the header (RFC 7518 4.7). */
async function wrapWithKey(
    content: Uint8Array<ArrayBuffer>,
    key: CryptoKey,
    environment: Pick<Environment, "crypto" | "random">,
) {
    const iv = environment.random.bytes(IV_BYTES);
    const sealed = new Uint8Array(
        await environment.crypto.encrypt({ name: "AES-GCM", iv }, key, content),
    );

    return {
        key: sealed.subarray(0, sealed.length - TAG_BYTES),
        members: {
            iv: Base64Url.encode(iv),
            tag: Base64Url.encode(sealed.subarray(sealed.length - TAG_BYTES)),
        },
    };
}

/** Unwrap a content key with an AES-GCM key, by the header's vector and tag. */
async function unwrapWithKey(
    wrapped: Uint8Array<ArrayBuffer>,
    wrap: { readonly iv: Uint8Array<ArrayBuffer>; readonly tag: Uint8Array<ArrayBuffer> },
    key: CryptoKey,
    crypto: Crypto,
): Promise<CryptoKey> {
    const content = await crypto.decrypt(
        { name: "AES-GCM", iv: wrap.iv },
        key,
        concat([wrapped, wrap.tag]),
    );

    return crypto.importKey("raw", content, CONTENT, false, ["decrypt"]);
}

/** Wrap a content key with a key agreed with an ephemeral key (RFC 7518 4.6). */
async function wrapWithAgreement(
    content: Uint8Array<ArrayBuffer>,
    recipient: CryptoKey,
    environment: Pick<Environment, "crypto" | "random">,
) {
    // agree a key-encrypting key with an ephemeral key pair
    const { crypto } = environment;
    const ephemeral = await crypto.generateKey(CURVE, true, ["deriveBits"]);
    const encrypting = await agree(ephemeral.privateKey, recipient, crypto);

    // wrap the content key, and publish the ephemeral key's point
    const contentKey = await crypto.importKey("raw", content, CONTENT, true, ["encrypt"]);
    const wrapped = new Uint8Array(await crypto.wrapKey("raw", contentKey, encrypting, "AES-KW"));
    const { kty, crv, x, y } = await crypto.exportKey("jwk", ephemeral.publicKey);

    return { key: wrapped, members: { epk: { kty, crv, x, y } } };
}

/** Unwrap a content key with a key agreed with the header's ephemeral key. */
async function unwrapWithAgreement(
    wrapped: Uint8Array<ArrayBuffer>,
    epk: EphemeralKey,
    privateKey: CryptoKey,
    crypto: Crypto,
): Promise<CryptoKey> {
    // agree the key with the header's ephemeral key, and unwrap the content key
    const ephemeral = await crypto.importKey("jwk", epk, CURVE, false, []);
    const encrypting = await agree(privateKey, ephemeral, crypto);

    return crypto.unwrapKey("raw", wrapped, encrypting, "AES-KW", CONTENT, false, ["decrypt"]);
}

/** Agree an A256KW key by ECDH and the Concat KDF over SHA-256 (RFC 7518 4.6.2). */
async function agree(
    privateKey: CryptoKey,
    publicKey: CryptoKey,
    crypto: Crypto,
): Promise<CryptoKey> {
    // agree the shared secret
    const shared = new Uint8Array(
        await crypto.deriveBits(
            { name: "ECDH", public: publicKey },
            privateKey,
            SHARED_SECRET_BYTES * 8,
        ),
    );

    // hash one round of the Concat KDF input for 256 bits
    const algorithm = new TextEncoder().encode("ECDH-ES+A256KW");
    const input = concat([
        bigEndian(1),
        shared,
        bigEndian(algorithm.length),
        algorithm,
        bigEndian(0),
        bigEndian(0),
        bigEndian(KEY_BYTES * 8),
    ]);
    const derived = await crypto.digest("SHA-256", input);

    return crypto.importKey("raw", derived, "AES-KW", false, ["wrapKey", "unwrapKey"]);
}

/** Read the vector and tag of an A256GCMKW header, refusing other lengths. */
function readWrap(members: Record<string, unknown>) {
    const { iv, tag } = readMember(WrapMembers, members, "key wrap");

    return {
        iv: sized(Base64Url.decode(iv), IV_BYTES, "key wrap vector"),
        tag: sized(Base64Url.decode(tag), TAG_BYTES, "key wrap tag"),
    };
}

/** Parse a header member by its schema, refusing an invalid one. */
function readMember<Value>(member: schema.Schema<Value>, value: unknown, label: string): Value {
    const parsed = member.safeParse(value);
    if (!parsed.success) {
        throw IdentityError.of("CIPHERTEXT_MALFORMED", `the ciphertext has an invalid ${label}`);
    }

    return parsed.data;
}

/** Refuse bytes of another length than a JWE segment's. */
function sized(
    bytes: Uint8Array<ArrayBuffer>,
    length: number,
    label: string,
): Uint8Array<ArrayBuffer> {
    if (bytes.length !== length) {
        throw IdentityError.of(
            "CIPHERTEXT_MALFORMED",
            `the ciphertext's ${label} has ${bytes.length} bytes, not ${length}`,
        );
    }

    return bytes;
}

/** Encode a 32-bit unsigned number big-endian. */
function bigEndian(value: number): Uint8Array {
    const bytes = new Uint8Array(4);
    new DataView(bytes.buffer).setUint32(0, value);

    return bytes;
}

/** Join byte arrays into one, allocated once. */
function concat(arrays: readonly Uint8Array[]): Uint8Array<ArrayBuffer> {
    // allocate the total length, then copy each array at its offset
    const bytes = new Uint8Array(arrays.reduce((length, array) => length + array.length, 0));
    let offset = 0;
    for (const array of arrays) {
        bytes.set(array, offset);
        offset += array.length;
    }

    return bytes;
}

/** Split a compact JWE into its five segments. */
function segments(compact: string): readonly [string, string, string, string, string] {
    // split at the dots, refusing any count but five
    const split = compact.split(".");
    const [header, key, iv, body, tag] = split;
    if (
        split.length !== 5 ||
        header === undefined ||
        key === undefined ||
        iv === undefined ||
        body === undefined ||
        tag === undefined
    ) {
        throw IdentityError.of("CIPHERTEXT_MALFORMED", "the ciphertext is not a compact JWE");
    }

    return [header, key, iv, body, tag];
}
