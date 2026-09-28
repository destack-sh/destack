import { canonicalize } from "@destack/schema/json";
import { schema } from "@destack/schema";
import { v7 } from "uuid";
import { ServiceError } from "../error/index.ts";

/** Maximum age of a retriable mutation, measured from its immutable UUID timestamp. */
export const REQUEST_LIFETIME_MS = 7 * 24 * 60 * 60 * 1000;
/** Maximum accepted clock difference for newly issued request keys. */
const CLOCK_TOLERANCE_MS = 5 * 60 * 1000;

/** A timestamped idempotency key retained across attempts of the same mutation. */
export const RequestId = {
    /** The schema of a request identifier, a UUIDv7. */
    schema: schema.uuidv7(),

    /** Create a request identifier once, before the first attempt. */
    create(): string {
        return v7();
    },

    /** Read the retry deadline of a request identifier, rejecting expired or future ones. */
    expiry(requestId: string, now = Date.now()): number {
        // read the creation time from the UUIDv7 key
        const key = RequestId.schema.parse(requestId);
        const createdAt = Number.parseInt(key.slice(0, 13).replaceAll("-", ""), 16);
        const expiresAt = createdAt + REQUEST_LIFETIME_MS;

        // reject keys outside the retry period, even after their stored responses are removed
        if (createdAt > now + CLOCK_TOLERANCE_MS || expiresAt <= now) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: "request identifier is outside its retry period",
            });
        }

        return expiresAt;
    },
};

/** The authenticated caller and scope containing a logical mutation. */
export interface RequestIdentity {
    /** Stable authenticated principal, independent of credential rotation. */
    readonly caller: string;
    /** Account, space or other authoritative scope. */
    readonly scope: string;
    /** The client-generated mutation identifier. */
    readonly requestId: string;
}

/** A fingerprint of a request's input: the SHA-256 digest of its canonical JSON without its sensitive values. */
export interface RequestFingerprint {
    /** The digest, as lowercase hexadecimal. */
    readonly digest: string;
}

/** A fingerprint of a request's input, which holds nothing derived from its sensitive values. */
export const RequestFingerprint = {
    /** Digest the canonical JSON of an input without the values its schema marks sensitive. */
    async hash(input: schema.Schema, value: unknown): Promise<RequestFingerprint> {
        const canonical = canonicalize(schema.json().parse(schema.redact(input, value) ?? null));
        const bytes = new TextEncoder().encode(canonical);

        return { digest: new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex() };
    },
};
