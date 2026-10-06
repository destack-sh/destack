import { defineSchema, schema } from "@destack/schema";
import { IdentityError } from "../error/error.ts";
import { PublicKey } from "../key/key.ts";
import { compactVerify, decodeJwt, decodeProtectedHeader, errors, SignJWT } from "jose";

/** The type a key proof declares in its header. */
const PROOF_TYPE = "key-proof+jwt";

/** How far a proof's issue time may lie from now, a round trip plus skew in seconds. */
const PROOF_SKEW_SECONDS = 60;

/** The longest proof a client or machine presents, far above the few hundred bytes of an ES256 compact JWS. */
const PROOF_LENGTH = 4096;

/** A key proof in its compact serialization, as clients and machines present it. */
export const CompactProof = defineSchema(schema.string().min(1).max(PROOF_LENGTH));

/** The header carrying a key's proof bound to the request it authenticates (RFC 9449 4.1). */
export const PROOF_HEADER = "dpop";

/** The protected header of a key proof. */
const ProofHeader = schema.object({
    /** The signing algorithm, ECDSA over P-256 with SHA-256. */
    alg: schema.literal("ES256"),
    /** The proof type. */
    typ: schema.literal(PROOF_TYPE),
    /** The RFC 7638 thumbprint of the signing key. */
    kid: schema.string().min(1),
});

/** The claims of a key proof, named as in RFC 9449. */
const ProofClaims = schema.object({
    /** The client or machine the proof speaks for. */
    sub: schema.string().min(1),
    /** The issue time in seconds since the epoch. */
    iat: schema.number().int(),
    /** The proof's unique identifier. */
    jti: schema.string().min(16).max(64),
    /** The HTTP method of the request a request proof authenticates. */
    htm: schema.string().min(1).exactOptional(),
    /** The URL, without query and fragment, of the request a request proof authenticates. */
    htu: schema.url().exactOptional(),
});

/** The request a request proof authenticates. */
export interface ProofRequest {
    /** The HTTP method. */
    readonly method: string;
    /** The URL. */
    readonly url: string;
}

/** A client's or machine's proof that it holds a key's private half, as a compact ES256 JWS, after RFC 9449. */
export class Proof {
    /** The RFC 7638 thumbprint of the signing key. */
    readonly thumbprint: string;
    /** The client or machine the proof speaks for. */
    readonly subject: string;
    /** The issue time in seconds since the epoch. */
    readonly issuedAt: number;
    /** The proof's unique identifier. */
    readonly id: string;
    /** The request the proof authenticates, absent for a possession proof. */
    readonly request: ProofRequest | undefined;
    /** The compact serialization its signature is verified over. */
    readonly #compact: string;

    /** Hold a proof's named key, claims and serialization. */
    private constructor(
        parts: Pick<Proof, "thumbprint" | "subject" | "issuedAt" | "id" | "request">,
        compact: string,
    ) {
        // keep the named key, claims and serialization
        this.thumbprint = parts.thumbprint;
        this.subject = parts.subject;
        this.issuedAt = parts.issuedAt;
        this.id = parts.id;
        this.request = parts.request;
        this.#compact = compact;
    }

    /** The last time a verifier accepts the proof, in UTC epoch milliseconds. */
    get expiresAt(): number {
        return (this.issuedAt + PROOF_SKEW_SECONDS) * 1000;
    }

    /** Read a proof's header and claims, refusing any other serialization, type or algorithm. */
    static read(proof: string): Proof {
        // require the declared type, algorithm, key and claims
        const header = ProofHeader.safeParse(decoded(() => decodeProtectedHeader(proof)));
        const claims = ProofClaims.safeParse(decoded(() => decodeJwt(proof)));
        if (!header.success || !claims.success) {
            throw invalid("key proof has an invalid header or claims");
        }

        // take the method and URL from a request proof
        const { htm, htu } = claims.data;
        if ((htm === undefined) !== (htu === undefined)) {
            throw invalid("key proof binds a method or a URL alone");
        }

        return new Proof(
            {
                thumbprint: header.data.kid,
                subject: claims.data.sub,
                issuedAt: claims.data.iat,
                id: claims.data.jti,
                request:
                    htm === undefined || htu === undefined ? undefined : { method: htm, url: htu },
            },
            proof,
        );
    }

    /** Sign a proof for a subject. */
    static async sign(
        privateKey: CryptoKey,
        publicKey: PublicKey,
        subject: string,
        now: number,
        request?: ProofRequest,
    ): Promise<string> {
        return new SignJWT({
            sub: subject,
            iat: Math.floor(now / 1000),
            jti: crypto.randomUUID(),
            ...(request === undefined
                ? {}
                : { htm: request.method, htu: requestTarget(request.url) }),
        })
            .setProtectedHeader({
                alg: "ES256",
                typ: PROOF_TYPE,
                kid: await PublicKey.thumbprint(publicKey),
            })
            .sign(privateKey);
    }

    /** Verify the proof with its signing key. */
    async verify(
        key: PublicKey,
        subject: string,
        now: number,
        request?: ProofRequest,
    ): Promise<void> {
        // require the named key's signature
        if (this.thumbprint !== (await PublicKey.thumbprint(key))) {
            throw invalid("key proof signature is invalid");
        }
        try {
            await compactVerify(this.#compact, await PublicKey.import(key), {
                algorithms: ["ES256"],
            });
        } catch (error) {
            if (!(error instanceof errors.JOSEError)) {
                throw error;
            }
            throw invalid("key proof signature is invalid", error);
        }

        // require the proof to speak for the subject and to be signed now
        const skew = Math.abs(now / 1000 - this.issuedAt);
        if (this.subject !== subject || skew > PROOF_SKEW_SECONDS) {
            throw invalid("key proof is for another subject or time");
        }

        // require the proof to bind exactly the request
        const isBound =
            request === undefined
                ? this.request === undefined
                : this.request?.method === request.method &&
                  this.request.url === requestTarget(request.url);
        if (!isBound) {
            throw invalid("key proof is for another request");
        }
    }
}

/** Refuse a malformed or unverifiable key proof. */
function invalid(message: string, cause?: unknown): IdentityError {
    return new IdentityError("INVALID_PROOF", message, cause === undefined ? undefined : { cause });
}

/** Decode a part of a compact JWS, refusing one that is no JWS. */
function decoded(decode: () => unknown): unknown {
    try {
        return decode();
    } catch (error) {
        if (!(error instanceof errors.JOSEError || error instanceof TypeError)) {
            throw error;
        }
        throw invalid("key proof is no compact JWS", error);
    }
}

/** Read a request's URL without its query and fragment. */
function requestTarget(url: string): string {
    const parsed = new URL(url);

    return `${parsed.origin}${parsed.pathname}`;
}
