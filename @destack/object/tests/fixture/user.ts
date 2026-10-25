import { type Identifier, present, schema } from "@destack/schema";
import { principal } from "@destack/access";
import type { RequestContext } from "@destack/object/server";
import { subjectContext, type SubjectContextOptions } from "@destack/object/test";

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
    options: SubjectContextOptions,
): RequestContext {
    return subjectContext(principal.user.reference("universe", user), scope, options);
}
