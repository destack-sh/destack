import { digest } from "@destack/schema/json";
import { schema } from "@destack/schema";
import { v7 } from "uuid";
import { ServiceError } from "../error/index.ts";

/** The header carrying the release of the callee's package a caller was built against, its API version. */
export const VERSION_HEADER = "Destack-Version";

/** The retry lifetime of a request identifier. */
export const REQUEST_LIFETIME_MILLISECONDS = 7 * 24 * 60 * 60 * 1000;
/** The clock tolerance for new request identifiers. */
const CLOCK_TOLERANCE_MILLISECONDS = 5 * 60 * 1000;

/** The originals of copied requests. */
const ORIGINALS = new WeakMap<Request, Request>();

/** Copy a request with changes and an optional URL and hold the client's original while the copy lives. */
export function copyRequest(request: Request, changes: RequestInit, url = request.url): Request {
    const copy = new Request(url, new Request(request, changes));
    ORIGINALS.set(copy, originalRequest(request));

    return copy;
}

/** Find a request as its client sent it, before any router copied it. */
export function originalRequest(request: Request): Request {
    return ORIGINALS.get(request) ?? request;
}

/** A timestamped request identifier kept across retries. */
export const RequestId = {
    /** The schema of a request identifier, a UUIDv7. */
    schema: schema.uuidv7(),

    /** Create a request identifier. */
    create(): string {
        return v7();
    },

    /** Read the retry deadline of a request identifier. */
    expiry(requestId: string, now = Date.now()): number {
        // read the creation time
        const key = RequestId.schema.parse(requestId);
        const createdAt = Number.parseInt(key.slice(0, 13).replaceAll("-", ""), 16);
        const expiresAt = createdAt + REQUEST_LIFETIME_MILLISECONDS;

        // reject keys outside the retry period
        if (createdAt > now + CLOCK_TOLERANCE_MILLISECONDS || expiresAt <= now) {
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

/** A fingerprint of a request's input. */
export const RequestFingerprint = {
    /** Digest an input's canonical JSON without its sensitive values. */
    async hash(input: schema.Schema, value: unknown): Promise<RequestFingerprint> {
        return { digest: await digest(schema.redact(input, value) ?? null) };
    },
};
