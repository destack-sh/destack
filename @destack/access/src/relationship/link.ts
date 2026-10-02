import { Digest } from "@destack/schema";
import { AccessError } from "../error/index.ts";

/** The form of a capability secret: 32 bytes as lowercase hexadecimal. */
const SECRET = /^[0-9a-f]{64}$(?![\s\S])/u;

/** A link: a relationship to anyone who presents its secret, revoked by deleting the relationship. */
export interface Link {
    /** The relationship the link is. */
    readonly id: string;
    /** The secret, shown once to the link's creator. */
    readonly secret: string;
}

/** An unguessable secret that admits its holder through a relationship, stored as a digest. */
export class Capability {
    /** The secret a request presents. */
    readonly secret: string;
    /** The digest relationships keep. */
    readonly digest: string;

    /** Pair a secret with its digest. */
    constructor(secret: string, digest: string) {
        this.secret = secret;
        this.digest = digest;
    }

    /** Create a new secret with its digest. */
    static async create(): Promise<Capability> {
        const secret = crypto.getRandomValues(new Uint8Array(32)).toHex();

        return new Capability(secret, await Capability.digest(secret));
    }

    /** Hash a presented secret into the digest requests carry and refuse unknown secrets. */
    static async digest(secret: string): Promise<string> {
        // reject secrets access never created
        if (!SECRET.test(secret)) {
            throw new AccessError("FORBIDDEN", "invalid capability");
        }

        // hash the secret
        return Digest.of(secret);
    }
}
