import type { Environment } from "@destack/environment";
import { Digest, type DigestCrypto } from "@destack/schema";
import { AccessError } from "../error/index.ts";

/** The form of a link secret: 32 bytes as lowercase hexadecimal. */
const SECRET = /^[0-9a-f]{64}$/u;

/** A link: a relationship to anyone presenting its secret, revoked by deleting it. */
export interface Link {
    /** The relationship the link is. */
    readonly id: string;
    /** The secret, shown once to the link's creator. */
    readonly secret: string;
}

/** An unguessable secret that allows its holder through a relationship, stored as a digest. */
export class LinkSecret {
    /** The secret a request presents. */
    readonly secret: string;
    /** The digest relationships store. */
    readonly digest: Digest;

    /** Pair a secret with its digest. */
    constructor(secret: string, digest: Digest) {
        this.secret = secret;
        this.digest = digest;
    }

    /** Create a new secret with its digest. */
    static async create(environment: Pick<Environment, "crypto" | "random">): Promise<LinkSecret> {
        const secret = environment.random.bytes(32).toHex();

        return new LinkSecret(secret, await LinkSecret.digest(secret, environment.crypto));
    }

    /** Hash a presented secret into the digest requests send and refuse unknown secrets. */
    static async digest(secret: string, crypto: DigestCrypto): Promise<Digest> {
        // reject secrets access never created
        if (!SECRET.test(secret)) {
            throw AccessError.of("LINK_SECRET_INVALID", "invalid link secret");
        }

        // hash the secret
        return Digest.of(secret, crypto);
    }
}
