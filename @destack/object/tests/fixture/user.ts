import { type Identifier, present, schema } from "@destack/schema";
import { type Caller, principal } from "@destack/access";
import type { PackageId } from "@destack/package";
import type { RequestContext } from "@destack/runtime/server";
import { subjectContext, type SubjectContextOptions } from "@destack/runtime/test";

/** The people object's tests sign in as, each by a fixed user identifier. */
export const users = {
    alice: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000100"),
    bob: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000101"),
    carol: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000102"),
    ada: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000103"),
    owner: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000104"),
    person: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000105"),
    other: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000106"),
    member: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000107"),
    frank: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000108"),
    grace: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000109"),
    heidi: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000010a"),
    ivan: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000010b"),
    judy: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000010c"),
    dave: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-00000000010d"),
};

/** Read the name of a user the tests sign in as, by its identifier. */
export function userName(id: string): string {
    const [name] = present(
        Object.entries(users).find(([, user]) => user === id),
        `a test user ${id}`,
    );

    return name;
}

/** Build the request context of a signed-in user calling in a scope or in none. */
export function userContext(
    user: Identifier<"user">,
    scope: string | undefined,
    options: SubjectContextOptions = {},
): RequestContext {
    return subjectContext(principal.user.reference("universe", user), scope, options);
}

/** Authenticate a request as the user of its bearer token, else as no one. */
export function bearerUser(
    audience: PackageId,
    scope: string,
): (request: Request) => Promise<Caller | undefined> {
    return async (request) => {
        // read the user in the bearer token
        const bearer = request.headers.get("authorization");
        if (bearer === null) {
            return undefined;
        }
        const user = schema.identifier("user").parse(bearer.slice("Bearer ".length));
        const subject = principal.user.reference("universe", user);
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
