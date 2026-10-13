import { type Caller, principal } from "@destack/access";
import type { PackageId } from "@destack/package";
import type { RequestContext } from "@destack/runtime/server";
import { subjectContext, type SubjectContextOptions } from "@destack/runtime/test";

/** Build the request context of a signed-in user calling in a scope or in none. */
export function userContext(
    user: string,
    scope: string | undefined,
    options: SubjectContextOptions = {},
): RequestContext {
    return subjectContext(principal.user.reference("universe", user), scope, options);
}

/** Authenticate a request as the user its bearer token names, verified for an audience in a scope for a minute, and as no one without a token. */
export function bearerUser(
    audience: PackageId,
    scope: string,
): (request: Request) => Promise<Caller | undefined> {
    return async (request) => {
        // read the user the bearer token names
        const bearer = request.headers.get("authorization");
        if (bearer === null) {
            return undefined;
        }
        const subject = principal.user.reference("universe", bearer.slice("Bearer ".length));
        const now = Date.now();

        return {
            subject,
            subjects: [subject],
            credential: {
                kind: "session",
                id: subject.id,
                audience,
                scope,
                verifiedAt: now,
                expiresAt: now + 60_000,
            },
        };
    };
}
