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

/** A canonical fingerprint of a request's input, optionally protected by a versioned encryption key. */
export interface RequestFingerprint {
    /** Serialized fingerprint; request secrets must be protected against offline guessing. */
    readonly digest: schema.Infer<ReturnType<typeof schema.json>>;
    /** Encryption key version indexed for rewrapping. */
    readonly keyId?: string;
}

/** A canonical fingerprint of a request's input, optionally protected by a versioned encryption key. */
export const RequestFingerprint = {
    /** Hash the canonical JSON of a request's input. */
    async hash(input: unknown): Promise<Uint8Array<ArrayBuffer>> {
        // hash the canonical input bytes, then clear them
        const bytes = new TextEncoder().encode(canonicalize(schema.json().parse(input)));
        try {
            return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
        } finally {
            bytes.fill(0);
        }
    },
};
