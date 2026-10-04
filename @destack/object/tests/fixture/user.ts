import { principal } from "@destack/access";
import type { ServiceContext } from "@destack/service/server";
import { subjectContext, type SubjectContextOptions } from "@destack/service/test";

/** Build the request context of a signed-in user calling in a scope or in none. */
export function userContext(
    user: string,
    scope: string | undefined,
    options: SubjectContextOptions = {},
): ServiceContext {
    return subjectContext(principal.user.reference("universe", user), scope, options);
}
