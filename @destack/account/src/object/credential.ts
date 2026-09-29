import { Digest } from "./digest.ts";

/** The prefix of each kind of secret. */
const PREFIXES = {
    "personal-access-token": "dst_pat_",
    "service-token": "dst_svc_",
    "oauth-client": "dst_ocs_",
} as const;

/** The random bytes of a secret, 256 bits. */
const SECRET_BYTES = 32;

/** The kind of secret. */
export type CredentialKind = keyof typeof PREFIXES;

/** The kind of token a bearer secret authenticates as. */
export type TokenKind = Exclude<CredentialKind, "oauth-client">;

/** A secret and the digest its object keeps. */
export class Credential {
    /** The kind of secret. */
    readonly kind: CredentialKind;
    /** The secret the client shows once and presents. */
    readonly secret: string;
    /** The digest the object keeps. */
    readonly digest: string;

    /** Pair a secret with its kind and digest. */
    private constructor(kind: CredentialKind, secret: string, digest: string) {
        this.kind = kind;
        this.secret = secret;
        this.digest = digest;
    }

    /** Create a new secret of a kind with its digest. */
    static async create(kind: CredentialKind): Promise<Credential> {
        // draw the secret's bytes after its kind's prefix
        const bytes = crypto.getRandomValues(new Uint8Array(SECRET_BYTES));
        const encoded = bytes.toBase64({ alphabet: "base64url", omitPadding: true });
        const secret = `${PREFIXES[kind]}${encoded}`;

        return new Credential(kind, secret, await Digest.hex(secret));
    }

    /** Read the kind of a presented secret, absent for other credentials such as sessions. */
    static kind(secret: string): CredentialKind | undefined {
        return (Object.keys(PREFIXES) as CredentialKind[]).find((kind) =>
            secret.startsWith(PREFIXES[kind]),
        );
    }
}
