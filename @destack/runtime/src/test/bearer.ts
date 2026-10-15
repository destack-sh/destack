import { type Authentication, Caller, principal } from "@destack/access";
import type { PackageId } from "@destack/package";
import type { Subject } from "@destack/sync";
import { SPACE_KEY, TOKEN_LIFETIME_MILLISECONDS } from "../authentication/index.ts";

/** The scheme a test request's authorization header puts its bearer under (RFC 6750 2.1). */
const BEARER_PREFIX = "Bearer ";

/** Test callers identifying themselves in a bearer, authenticated without a signature. */
export const TestBearer = {
    /** Read the bearer of a test request, refusing a request without one. */
    read(request: Request): string {
        const header = request.headers.get("authorization");
        if (header === null || !header.startsWith(BEARER_PREFIX)) {
            throw new TypeError("a test request has no authorization");
        }

        return header.slice(BEARER_PREFIX.length);
    },

    /** Authenticate a subject for a minute. */
    authenticate(subject: Subject, audience: PackageId, now: number): Authentication {
        const kind = principal.space.is(subject) ? SPACE_KEY : subject.type;

        return {
            caller: Caller.of(subject),
            credential: {
                kind,
                id: subject.id,
                audience,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
            },
        };
    },
};
