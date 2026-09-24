import { schema } from "@destack/schema";
import { v7 } from "uuid";
import { ServiceError } from "../error/index.ts";

/** Maximum age of a retriable mutation, measured from its immutable UUID timestamp. */
export const REQUEST_LIFETIME_MS = 7 * 24 * 60 * 60 * 1000;
/** Maximum accepted clock difference for newly issued request keys. */
const CLOCK_TOLERANCE_MS = 5 * 60 * 1000;

/** A timestamped idempotency key retained across attempts of the same mutation. */
export const RequestId = schema.uuidv7();

/** The authenticated scope and procedure containing a logical mutation. */
export interface RequestIdentity {
    /** Stable authenticated principal, independent of credential rotation. */
    readonly caller: string;
    /** Account, space or other authoritative scope. */
    readonly scope: string;
    /** Stable procedure identifier within the service. */
    readonly procedure: string;
    /** The client-generated mutation identifier. */
    readonly requestId: string;
}

/** Create a mutation key once, before the first attempt. */
export function createRequestId(): string {
    return v7();
}

/** Reject expired or future mutation keys even after their stored responses are removed. */
export function requestExpiry(requestId: string, now = Date.now()): number {
    // read the creation time from the UUIDv7 key and reject keys outside the retry period
    const key = RequestId.parse(requestId);
    const createdAt = Number.parseInt(key.slice(0, 13).replaceAll("-", ""), 16);
    const expiresAt = createdAt + REQUEST_LIFETIME_MS;
    if (createdAt > now + CLOCK_TOLERANCE_MS || expiresAt <= now) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: "request identifier is outside its retry period",
        });
    }

    return expiresAt;
}

/** Hash canonical JSON input before storing or protecting its fingerprint. */
export async function fingerprintRequest(input: unknown): Promise<Uint8Array<ArrayBuffer>> {
    const bytes = new TextEncoder().encode(JSON.stringify(schema.json().parse(input), sortObject));
    try {
        return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
    } finally {
        bytes.fill(0);
    }
}

/** Order object properties while retaining array order and scalar values. */
function sortObject(_name: string, value: unknown): unknown {
    if (value === null || typeof value !== "object" || Array.isArray(value)) {
        return value;
    }

    return Object.fromEntries(
        Object.entries(value).sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0)),
    );
}
