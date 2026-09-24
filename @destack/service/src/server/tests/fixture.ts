import { PackageId } from "@destack/package";
import { Caller } from "../../authentication/index.ts";
import type { Subject } from "@destack/access";
import { ResourceContext } from "@destack/resource/context";
import { ServiceError } from "../../error/index.ts";

/** Trusted hosting configuration for HTTP lifecycle and operation scenarios. */
export const hosting = {
    audience: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
    scope: "test-space",
    resources: new ResourceContext(),
    authenticate: async (request: Request) => {
        const name = request.headers.get("authorization");
        if (name !== "alice" && name !== "bob") {
            throw new ServiceError("UNAUTHORIZED");
        }

        return createCaller(name);
    },
    authorizeHost: async () => {},
    authorize: async () => {},
};

/** Construct a fresh verified identity for a known fixture user. */
export function createCaller(name: string): Caller {
    const subject: Subject = { kind: "user", authority: "global", id: name };
    const now = Date.now();

    return new Caller({
        subject,
        subjects: [subject],
        credential: { kind: "user", id: name },
        audience: hosting.audience,
        scope: hosting.scope,
        verifiedAt: now,
        expiresAt: now + 60000,
    });
}
