import { type Caller, principal } from "@destack/access";
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
            throw new CallError("UNAUTHORIZED", { message: "invalid caller name" });
        }

        return createCaller(name);
    },
    authorizeMachine: async () => {},
};

/** Create a verified caller for a fixture user. */
export function createCaller(name: string): Caller {
    const subject: Subject = principal.user.reference("universe", name);
    const now = Date.now();

    return {
        subject,
        subjects: [subject],
        credential: {
            kind: "session",
            id: name,
            audience: hosting.package.id,
            scope: hosting.scope,
            verifiedAt: now,
            expiresAt: now + 60000,
        },
    };
}
