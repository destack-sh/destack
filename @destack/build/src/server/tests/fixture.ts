import { copyOwner, copyScope } from "@destack/access/test";
import { PackageId } from "@destack/package";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { Caller } from "@destack/service/authentication";
import { ResourceContext } from "@destack/resource/context";
import { ServiceError } from "@destack/service/error";
import {
    ACCESS_TABLES,
    accessRelationship,
    accessScope,
    Authorizer,
    Relationship,
    principal,
} from "@destack/access";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import type { ServiceAccess } from "@destack/service/server";
import { BUILD_POLICIES } from "../../access/index.ts";

/** Trusted hosting configuration for build and preview HTTP scenarios. */
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
};

/** Construct a fresh verified identity for a known fixture user. */
export function createCaller(name: string): Caller {
    const subject = principal.user.reference("global", name);
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

/** Open the fixture scope's access, which alice owns and bob holds the owner role of. */
export async function openAccess(): Promise<{ access: ServiceAccess } & AsyncDisposable> {
    // record the fixture scope, a user's own, and its owners in a migrated database
    const test = await TestDatabase.create(TEST_DIALECTS.at(-1)!, ACCESS_TABLES, {
        isMigrated: true,
    });
    const scope = principal.user.reference("global", hosting.scope);
    const now = Date.now();
    await copyScope(test.database, scope);
    const owner = await copyOwner(
        test.database,
        scope,
        principal.user.reference("global", "alice"),
        now,
    );

    // decide the build package's permissions through roles in the fixture scope
    const authorizer = new Authorizer(BUILD_POLICIES, [
        {
            policy: principal.user,
            table: accessScope,
            id: "scope",
            scope: "parent",
            isShared: true,
            attributes: {},
            relations: {},
        },
    ]);
    await test.database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: identifier("relationship").parse(`relationship-${v7()}`),
                object: scope,
                role: owner,
                subject: principal.user.reference("global", "bob"),
                createdAt: now,
                expiresAt: null,
            },
            scope.id,
        ),
    );

    return {
        access: {
            authorizer,
            database: test.database,
            target: async () => scope,
        },
        [Symbol.asyncDispose]: () => test.close(),
    };
}
