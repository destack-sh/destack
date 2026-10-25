import type { Environment } from "@destack/environment";
import { Digest } from "@destack/schema";

/** The kinds of secret: presented credentials, and one-time codes and tokens of sign-ins. */
const CREDENTIAL_KINDS = [
    "personal-access-token",
    "client-secret",
    "refresh-token",
    "authorization-code",
    "device-code",
    "challenge",
] as const;

/** The prefix of each kind of secret, which secret scanners recognize, as GitHub's tokens have. */
const PREFIXES: { readonly [Kind in CredentialKind]: string } = {
    "personal-access-token": "dst_pat_",
    "client-secret": "dst_ocs_",
    "refresh-token": "dst_rt_",
    "authorization-code": "dst_ac_",
    "device-code": "dst_dc_",
    challenge: "dst_ch_",
};

/** The random bytes of a secret, 256 bits. */
const SECRET_BYTES = 32;

/** The kind of secret. */
export type CredentialKind = (typeof CREDENTIAL_KINDS)[number];

/** A secret and the digest its object stores. */
export class Credential {
    /** The kind of secret. */
    readonly kind: CredentialKind;
    /** The secret the client shows once and presents. */
    readonly secret: string;
    /** The digest the object stores. */
    readonly digest: Digest;

    /** Pair a secret with its kind and digest. */
    private constructor(kind: CredentialKind, secret: string, digest: Digest) {
        this.kind = kind;
        this.secret = secret;
        this.digest = digest;
    }

    /** Create a new secret of a kind with its digest, drawn from an environment's randomness. */
    static async create(
        kind: CredentialKind,
        environment: Pick<Environment, "random" | "crypto">,
    ): Promise<Credential> {
        // draw the secret's bytes after its kind's prefix
        const bytes = environment.random.bytes(SECRET_BYTES);
        const encoded = bytes.toBase64({ alphabet: "base64url", omitPadding: true });
        const secret = `${PREFIXES[kind]}${encoded}`;

        return new Credential(kind, secret, await Digest.of(secret, environment.crypto));
    }

    /** Read the kind of a presented secret, absent for other credentials such as sessions. */
    static kind(secret: string): CredentialKind | undefined {
        return CREDENTIAL_KINDS.find((kind) => secret.startsWith(PREFIXES[kind]));
    }
}
