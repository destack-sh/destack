import { principal } from "@destack/access";
import type { Subject } from "@destack/sync";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Authentication } from "../../authentication/index.ts";
import { ServiceError } from "../../error/index.ts";
import { defineService } from "../../declare/index.ts";

/** The hosting configuration of the server scenarios. */
export const hosting = {
    service: defineService("fixture", {}),
    audience: PackageId.parse("package-019f7480-0000-7000-8000-000000000001"),
    scope: "test-space",
    resources: new ResourceContext(),
    authenticate: async (request: Request) => {
        const name = request.headers.get("authorization");
        if (name !== "alice" && name !== "bob") {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid caller name" });
        }

        return createAuthentication(name);
    },
    authorizeHost: async () => {},
};

/** Create a verified caller for a fixture user. */
export function createAuthentication(name: string): Authentication {
    const subject: Subject = principal.user.reference("universe", name);
    const now = Date.now();

    return new Authentication({
        subject,
        subjects: [subject],
        credential: { kind: "user", id: name },
        audience: hosting.audience,
        scope: hosting.scope,
        verifiedAt: now,
        expiresAt: now + 60000,
    });
}
