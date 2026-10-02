import { check, index, lte, sql, unique, type DatabaseConnection, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { user } from "./user.ts";
import { revokeOnce } from "./revocation.ts";
import { Digest } from "./digest.ts";
import { authenticationReplay } from "../stack/authentication/key.ts";

/** The type a device key proof declares in its header. */
const PROOF_TYPE = "device-key+jwt";

/** How far a proof's issue time may lie from now, a round trip plus skew in seconds. */
const PROOF_SKEW_SECONDS = 60;

/** A device's P-256 public key as a JSON Web Key. */
export const DevicePublicKey = defineSchema(
    schema
        .object({
            /** The key type. */
            kty: schema.literal("EC"),
            /** The curve. */
            crv: schema.literal("P-256"),
            /** The x coordinate, base64url encoded. */
            x: schema.string().regex(/^[A-Za-z0-9_-]{43}$/),
            /** The y coordinate, base64url encoded. */
            y: schema.string().regex(/^[A-Za-z0-9_-]{43}$/),
        })
        .strict(),
);
/** A device's public key. */
export type DevicePublicKey = schema.Infer<typeof DevicePublicKey>;

/** The protected header of a device key proof. */
const ProofHeader = schema.object({
    /** The signing algorithm, ECDSA over P-256 with SHA-256. */
    alg: schema.literal("ES256"),
    /** The proof type. */
    typ: schema.literal(PROOF_TYPE),
    /** The RFC 7638 thumbprint of the signing key. */
    kid: schema.string().min(1),
});

/** The claims of a device key proof, named as in RFC 9449. */
const ProofClaims = schema.object({
    /** The device or host the proof speaks for. */
    sub: schema.string().min(1),
    /** The issue time in seconds since the epoch. */
    iat: schema.number().int(),
    /** The proof's unique identifier. */
    jti: schema.string().min(16).max(64),
    /** The HTTP method of the request a request proof authenticates. */
    htm: schema.string().min(1).optional(),
    /** The URL, without query and fragment, of the request a request proof authenticates. */
    htu: schema.url().optional(),
});

/** The request a request proof authenticates. */
export interface ProofRequest {
    /** The HTTP method. */
    readonly method: string;
    /** The URL. */
    readonly url: string;
}

/** A device's proof that it holds a key's private half, as a compact ES256 JWS. */
export class DeviceProof {
    /** The RFC 7638 thumbprint of the signing key. */
    readonly thumbprint: string;
    /** The device the proof speaks for. */
    readonly deviceId: string;
    /** The issue time in seconds since the epoch. */
    readonly issuedAt: number;
    /** The proof's unique identifier. */
    readonly id: string;
    /** The request the proof authenticates, absent for a possession proof. */
    readonly request: ProofRequest | undefined;
    /** The signed header and claims. */
    readonly signed: Uint8Array<ArrayBuffer>;
    /** The signature over them. */
    readonly signature: Uint8Array<ArrayBuffer>;

    /** Hold a proof's parts. */
    private constructor(parts: Omit<DeviceProof, "verify" | "consume">) {
        // take the named key, the claims and the signed bytes
        this.thumbprint = parts.thumbprint;
        this.deviceId = parts.deviceId;
        this.issuedAt = parts.issuedAt;
        this.id = parts.id;
        this.request = parts.request;
        this.signed = parts.signed;
        this.signature = parts.signature;
    }

    /** Read a proof's parts and refuse any other serialization, type or algorithm. */
    static read(proof: string): DeviceProof {
        // read the three parts of the compact serialization
        const parts = proof.split(".");
        if (parts.length !== 3) {
            throw invalid("device proof is no compact JWS");
        }
        const [encodedHeader, encodedClaims, encodedSignature] = parts as [string, string, string];

        // require the declared type, algorithm and key, and the claims
        const header = ProofHeader.safeParse(decodeJson(encodedHeader));
        const claims = ProofClaims.safeParse(decodeJson(encodedClaims));
        if (!header.success || !claims.success) {
            throw invalid("device proof has an invalid header or claims");
        }

        // take the method and URL from a request proof
        const { htm, htu } = claims.data;
        if ((htm === undefined) !== (htu === undefined)) {
            throw invalid("device proof binds a method or a URL alone");
        }

        return new DeviceProof({
            thumbprint: header.data.kid,
            deviceId: claims.data.sub,
            issuedAt: claims.data.iat,
            id: claims.data.jti,
            request: htm === undefined ? undefined : { method: htm, url: htu! },
            signed: new TextEncoder().encode(`${encodedHeader}.${encodedClaims}`),
            signature: decodeBytes(encodedSignature),
        });
    }

    /** Sign a proof for a subject. */
    static async sign(
        privateKey: CryptoKey,
        publicKey: DevicePublicKey,
        subject: string,
        now: number,
        request?: ProofRequest,
    ): Promise<string> {
        // encode the header with the key and the claims with the subject and the time
        const header = {
            alg: "ES256",
            typ: PROOF_TYPE,
            kid: await DeviceProof.thumbprint(publicKey),
        };
        const claims = {
            sub: subject,
            iat: Math.floor(now / 1000),
            jti: crypto.randomUUID(),
            ...(request === undefined
                ? {}
                : { htm: request.method, htu: requestTarget(request.url) }),
        };
        const signed = `${encodeJson(header)}.${encodeJson(claims)}`;

        // sign the header and claims with ECDSA over P-256 and SHA-256
        const signature = await crypto.subtle.sign(
            { name: "ECDSA", hash: "SHA-256" },
            privateKey,
            new TextEncoder().encode(signed),
        );

        return `${signed}.${new Uint8Array(signature).toBase64({ alphabet: "base64url", omitPadding: true })}`;
    }

    /** Compute a key's RFC 7638 thumbprint. */
    static async thumbprint(key: DevicePublicKey): Promise<string> {
        // hash the required members in lexicographic order
        const { crv, kty, x, y } = key;

        return Digest.base64url(JSON.stringify({ crv, kty, x, y }));
    }

    /** Verify the proof with its signing key. */
    async verify(
        key: DevicePublicKey,
        deviceId: string,
        now: number,
        request?: ProofRequest,
    ): Promise<void> {
        // require the named key's signature over the header and claims
        const imported = await crypto.subtle.importKey(
            "jwk",
            key,
            { name: "ECDSA", namedCurve: "P-256" },
            false,
            ["verify"],
        );
        const isSigned = await crypto.subtle.verify(
            { name: "ECDSA", hash: "SHA-256" },
            imported,
            this.signature,
            this.signed,
        );
        if (this.thumbprint !== (await DeviceProof.thumbprint(key)) || !isSigned) {
            throw invalid("device proof signature is invalid");
        }

        // require the proof to speak for the device and to be signed now
        const skew = Math.abs(now / 1000 - this.issuedAt);
        if (this.deviceId !== deviceId || skew > PROOF_SKEW_SECONDS) {
            throw invalid("device proof is for another device or time");
        }

        // require the proof to bind exactly the request
        const isBound =
            request === undefined
                ? this.request === undefined
                : this.request?.method === request.method &&
                  this.request.url === requestTarget(request.url);
        if (!isBound) {
            throw invalid("device proof is for another request");
        }
    }

    /** Use the proof once. */
    async consume(database: DatabaseConnection, now: number): Promise<void> {
        // forget the uses past every proof lifetime
        await database.delete(authenticationReplay).where(lte(authenticationReplay.expiresAt, now));

        // record the key's use of the identifier and refuse a repeat
        const digest = await Digest.hex(JSON.stringify([this.thumbprint, this.id]));
        const [recorded] = await database
            .insert(authenticationReplay)
            .values({
                id: digest,
                expiresAt: (this.issuedAt + PROOF_SKEW_SECONDS) * 1000,
            })
            .onConflictDoNothing()
            .returning({ id: authenticationReplay.id });
        if (recorded === undefined) {
            throw invalid("device proof was used before");
        }
    }
}

/** Attach the calling session to a device. */
const attach = method({
    permission: "update",
    input: schema.object({ proof: schema.string().min(1).max(4096) }),
});

/** Withdraw a device with its keys and sessions. */
const revoke = method({ permission: "revoke" });

/** A device a user registered. */
export const device = defineObject({
    name: "device",
    tier: "global",
    plural: "devices",
    scope: user,
    fields: {
        /** The user's name for the device. */
        name: field.string(schema.string().min(1).max(200)),
        /** The form of the device. */
        category: field
            .enum(["desktop", "laptop", "phone", "tablet", "server", "unknown"])
            .default("unknown"),
        /** The operating system, such as macos. */
        operatingSystem: field.string().optional(),
        /** The operating system's version. */
        operatingSystemVersion: field.string().optional(),
        /** The processor architecture, such as arm64. */
        architecture: field.string().optional(),
        /** The hardware model. */
        model: field.string().optional(),
        /** The version of the Destack client that registered the device. */
        clientVersion: field.string().optional(),
        /** The last time the device proved itself. */
        lastSeenAt: field.time().optional(),
        /** The time the user withdrew the device. */
        revokedAt: field.time().optional(),
    },
    permissions: ["read", "create", "update", "revoke"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", {
            fields: [
                "name",
                "category",
                "operatingSystem",
                "operatingSystemVersion",
                "architecture",
                "model",
                "clientVersion",
            ],
            isPredicted: false,
        }),
        update: method.update("update", { fields: ["name"] }),
        attach,
        revoke,
        report: method({ permission: null, isSystem: true }).handle((call) =>
            // record the device's proven contact now
            call.update({ lastSeenAt: call.now }),
        ),
    },
    constraints: (device) => [index("device_scope").on(device.scope)],
});
/** A persisted device record. */
export type Device = Select<typeof device.table>;

/** A device's revocable authentication key. */
export const deviceKey = defineObject({
    name: "device-key",
    tier: "global",
    plural: "deviceKeys",
    scope: user,
    nested: { in: device, receive: "update", delete: "cascade" },
    fields: {
        /** The public key. */
        publicKey: field.json(DevicePublicKey),
        /** The RFC 7638 JWK thumbprint binding device proofs and tokens to the key. */
        thumbprint: field.string(schema.string().min(1)),
        /** The time the device proved possession of the private key. */
        verifiedAt: field.time(),
        /** The time the key stops authenticating. */
        expiresAt: field.time(),
        /** The time the user withdrew the key. */
        revokedAt: field.time().optional(),
    },
    permissions: ["read", "register", "revoke"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("register", {
            fields: ["publicKey"],
            input: schema.object({ proof: schema.string().min(1).max(4096) }),
            isPredicted: false,
        }),
        revoke: method({ permission: "revoke" }).handle((call) => revokeOnce(call, "device key")),
    },
    constraints: (key) => [
        unique("device_key_thumbprint").on(key.thumbprint),
        unique("device_key_device_id").on(key.parentId, key.id),
        check("device_key_expiry", sql`${key.expiresAt} > ${key.createdAt}`),
        check(
            "device_key_verification",
            sql`${key.verifiedAt} >= ${key.createdAt} AND ${key.verifiedAt} < ${key.expiresAt}`,
        ),
    ],
});
/** A device authentication key. */
export type DeviceKey = Select<typeof deviceKey.table>;

/** Refuse a malformed or unverifiable device proof. */
function invalid(message: string, cause?: unknown): Error {
    return new ServiceError("BAD_REQUEST", { message, ...(cause === undefined ? {} : { cause }) });
}

/** Read a request's URL without its query and fragment. */
function requestTarget(url: string): string {
    const parsed = new URL(url);

    return `${parsed.origin}${parsed.pathname}`;
}

/** Encode a value as a base64url JSON part of a JWS. */
function encodeJson(value: unknown): string {
    return new TextEncoder()
        .encode(JSON.stringify(value))
        .toBase64({ alphabet: "base64url", omitPadding: true });
}

/** Decode a base64url part of a JWS into JSON and refuse a part without JSON. */
function decodeJson(part: string): unknown {
    const text = new TextDecoder().decode(decodeBytes(part));
    try {
        return JSON.parse(text);
    } catch (error) {
        throw invalid("device proof part is no JSON", error);
    }
}

/** Decode a base64url part of a JWS into bytes and refuse any other encoding. */
function decodeBytes(part: string): Uint8Array<ArrayBuffer> {
    try {
        return Uint8Array.fromBase64(part, { alphabet: "base64url" });
    } catch (error) {
        throw invalid("device proof part is no base64url", error);
    }
}
