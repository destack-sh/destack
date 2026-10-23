import {
    type Authentication,
    type AuthenticationAssurance,
    Caller,
    principal,
} from "@destack/access";
import type { PackageId } from "@destack/package";
import { Subject } from "@destack/sync";
import { SPACE_KEY, TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";

/** The scheme a test request's authorization header puts its bearer under (RFC 6750 2.1). */
const BEARER_PREFIX = "Bearer ";

/** The header of a test request whose caller stepped up just now (RFC 9470 step-up). */
const STEP_UP_HEADER = "x-destack-step-up";

/** The assurance of a second factor, NIST SP 800-63B's AAL2. */
const AAL2 = 2;

/** Test callers identifying themselves in a bearer, authenticated without a signature. */
export const TestBearer = {
    /** Write the authorization header a subject's test requests send: its key as the bearer. */
    header(subject: Subject): string {
        return `${BEARER_PREFIX}${Subject.key(subject)}`;
    },

    /** Write a subject's test request headers, with a fresh second factor once stepped up. */
    headers(subject: Subject, isSteppedUp: boolean): Record<string, string> {
        return {
            authorization: TestBearer.header(subject),
            ...(isSteppedUp ? { [STEP_UP_HEADER]: "1" } : {}),
        };
    },

    /** Read a test request's assurance: a second factor as it arrives, once stepped up. */
    assurance(request: Request, now: number): AuthenticationAssurance | undefined {
        return request.headers.has(STEP_UP_HEADER)
            ? { level: AAL2, authenticatedAt: now }
            : undefined;
    },

    /** Read the subject a test request's bearer is the key of, refusing a request without one. */
    subject(request: Request): Subject {
        return Subject.read(TestBearer.read(request));
    },

    /** Read the bearer of a test request, refusing a request without one. */
    read(request: Request): string {
        const header = request.headers.get("authorization");
        if (header === null || !header.startsWith(BEARER_PREFIX)) {
            throw new TypeError("a test request has no authorization");
        }

        return header.slice(BEARER_PREFIX.length);
    },

    /** Authenticate a subject for a token's lifetime, at an assurance when given. */
    authenticate(
        subject: Subject,
        audience: PackageId,
        now: number,
        assurance?: AuthenticationAssurance,
    ): Authentication {
        const kind = principal.space.is(subject) ? SPACE_KEY : subject.type;

        return {
            caller: Caller.of(subject),
            credential: {
                kind,
                id: subject.id,
                audience,
                verifiedAt: now,
                expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                ...(assurance === undefined ? {} : { assurance }),
            },
        };
    },
};
