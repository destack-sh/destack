import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/access/token";
import { type Authentication, Caller, principal } from "@destack/access";
import type { Subject } from "@destack/sync";
import { Package } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { CallError } from "../error/index.ts";
import { PackageHandle } from "../declare/index.ts";

/** The package the server scenarios host. */
export const fixturePackage = PackageHandle.of(
    Package.parse({
        id: "package-019f7480-0000-7000-8000-000000000001",
        name: "@destack/fixture",
        version: "2026.9.0",
    }),
    {},
);

/** The hosting configuration of the server scenarios. */
export const hosting = {
    package: fixturePackage,
    scope: "test-space",
    resources: new ResourceContext(),
    authenticate: async (request: Request) => {
        const name = request.headers.get("authorization");
        if (name !== "alice" && name !== "bob") {
            throw CallError.http("UNAUTHORIZED", { message: "invalid caller name" });
        }

        return authenticateUser(name);
    },
    authorizeMachine: async () => {},
};

/** Authenticate a fixture user with a session for a minute. */
export function authenticateUser(name: string): Authentication {
    const subject: Subject = principal.user.reference("universe", name);
    const now = Date.now();

    return {
        caller: Caller.of(subject),
        credential: {
            kind: "session",
            id: name,
            audience: hosting.package.id,
            scope: hosting.scope,
            verifiedAt: now,
            expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
        },
    };
}
