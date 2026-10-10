import { schema } from "@destack/schema";
import { v7 } from "uuid";
import { ServiceError } from "../error/index.ts";

/** The header with the release of the callee's package a caller was built against, its API version. */
export const VERSION_HEADER = "Destack-Version";

/** The retry lifetime of a request identifier. */
export const REQUEST_LIFETIME_MILLISECONDS = 7 * 24 * 60 * 60 * 1000;
/** How far a client's clock may run ahead when it creates a request identifier: five minutes, as Kerberos tolerates client skew. */
const CLIENT_CLOCK_TOLERANCE_MILLISECONDS = 5 * 60 * 1000;

/** The version digit of a time-ordered request identifier: 7 created, 8 derived (RFC 9562 5.7 and 5.8). */
const TIME_ORDERED = /^.{14}[78]/u;

/** The version digit of a derived request identifier (RFC 9562 5.8). */
const DERIVED = /^.{14}8/u;

/** The characters of a time-ordered UUID's leading time and its dash: 48 bits as "xxxxxxxx-xxxx-". */
const TIME_CHARACTERS = 14;

/** The bytes of a UUID. */
const UUID_BYTES = 16;

/** The bytes of the 48-bit Unix millisecond time leading a time-ordered UUID (RFC 9562 5.7). */
const TIME_BYTES = 6;

/** The version 8 nibble, in the high half of a UUID's seventh byte (RFC 9562 4.2). */
const VERSION_8 = 0x80;

/** The RFC 9562 variant, in the two high bits of a UUID's ninth byte (RFC 9562 4.1). */
const VARIANT = 0x80;

/** Copy a request with changes and an optional URL, building the copy once from the request's fields. */
export function copyRequest(request: Request, changes: RequestInit, url = request.url): Request {
    const body = changes.body === undefined ? request.body : changes.body;

    return new Request(url, {
        method: changes.method ?? request.method,
        headers: changes.headers ?? request.headers,
        body,
        signal: changes.signal ?? request.signal,
        redirect: changes.redirect ?? request.redirect,
        ...(body instanceof ReadableStream ? { duplex: "half" } : {}),
    });
}

/** A timestamped request identifier kept across retries as its idempotency key. */
export const RequestId = {
    /** The schema of a request identifier: a UUIDv7 a caller created, or a UUIDv8 it derived. */
    schema: schema.uuid().regex(TIME_ORDERED),

    /** Create a request identifier stamped with a time, the current time by default, which its retry period starts from. */
    create(time = Date.now()): string {
        return v7({ msecs: time });
    },

    /** Derive the request identifier of a key at a time, the same for every repeat: a time-ordered UUIDv8 (RFC 9562 5.8). */
    async derive(time: number, key: string): Promise<string> {
        // lead with the 48-bit time and fill the rest from the key's SHA-256 digest
        const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(key));
        const bytes = new Uint8Array(UUID_BYTES);
        const view = new DataView(bytes.buffer);
        view.setBigUint64(0, BigInt(time) << 16n);
        bytes.set(new Uint8Array(digest, 0, UUID_BYTES - TIME_BYTES), TIME_BYTES);

        // mark the version and the variant
        view.setUint8(6, (view.getUint8(6) & 0x0f) | VERSION_8);
        view.setUint8(8, (view.getUint8(8) & 0x3f) | VARIANT);
        const hex = bytes.toHex();

        return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
    },

    /** Read the key a request identifier names its request by: a derived one's key digest, a created one whole. */
    key(requestId: string): string {
        return DERIVED.test(requestId) ? requestId.slice(TIME_CHARACTERS) : requestId;
    },

    /** Read the retry deadline of a request identifier. */
    expiry(requestId: string, now: number): number {
        // read the time leading a created or derived identifier
        const key = RequestId.schema.parse(requestId);
        const createdAt = Number.parseInt(key.slice(0, 13).replaceAll("-", ""), 16);
        const expiresAt = createdAt + REQUEST_LIFETIME_MILLISECONDS;

        // reject keys outside the retry period
        if (createdAt > now + CLIENT_CLOCK_TOLERANCE_MILLISECONDS || expiresAt <= now) {
            throw new ServiceError("PRECONDITION_FAILED", {
                message: "request identifier is outside its retry period",
            });
        }

        return expiresAt;
    },
};

/** The caller and scope of a request. */
export interface RequestIdentity {
    /** The authenticated principal. */
    readonly caller: string;
    /** The account, space or other scope. */
    readonly scope: string;
    /** The request identifier. */
    readonly requestId: string;
}

/** A fingerprint of a request's input. */
export interface RequestFingerprint {
    /** The digest, as lowercase hexadecimal. */
    readonly digest: string;
}
