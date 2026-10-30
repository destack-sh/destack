import { testSimulation } from "../test/world.ts";
import { Package } from "@destack/package";
import { TestAccess } from "@destack/access/test";
import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Authorizer, Policy, principal, relation, Caller } from "@destack/access";
import { defineTable, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Health } from "./health.ts";
import { defineProcedure } from "../procedure/index.ts";
import { createClient } from "../client/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./http.ts";
import type { RequestContext } from "./context.ts";
import { fixturePackage, hosting } from "../test/fixture.ts";
import { tablesOf } from "../index.ts";

/** The authentication renaming a vault requires. */
const RECENT = { assurance: 2, maxAge: 10 * 60 * 1000 };

test("challenge a hand-written procedure's caller for its elevated permission, then allow it once it authenticates again", async () => {
    const world = testSimulation().environment("test");
    // declare a vault its owner renames after recent strong authentication
    const module = {
        package: { id: hosting.package.id, name: "@example/vault", version: "2026.9.0" },
    };
    const vault = new Policy(module.package, {
        name: "vault",
        relations: { owner: { subjects: [principal.user] } },
        permissions: { rename: relation("owner") },
        elevated: { rename: RECENT },
    });
    const space = new Policy(module.package, { name: "space", permissions: {}, scope: true });
    const vaults = defineTable("vault", {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        owner: text("owner").notNull(),
    });
    const authorizer = new Authorizer(
        [vault, space],
        [
            {
                policy: vault,
                table: vaults,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: { owner: { column: "owner", scope: "universe" } },
            },
        ],
    );

    // open a database for alice's vault
    const storage = await TestDatabase.create(
        tablesOf({ packages: [], follows: [], tables: [vaults] }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await new TestAccess(storage.database, world).replicateScope(
        {
            packageId: hosting.package.id,
            type: "space",
            scope: "universe",
            id: hosting.scope,
        },
        space,
    );
    await storage.database
        .insert(vaults)
        .values({ id: "main", scope: hosting.scope, owner: "alice" });

    // serve renaming with the permission
    const procedures = {
        rename: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: vault.permission("rename"),
            audit: false,
        })
            .route({ method: "POST", path: "/vaults/{id}/rename" })
            .input(schema.object({ id: schema.string() }))
            .output(schema.string()),
    };
    const implementation = implement(procedures).$context<RequestContext>();
    let level = 1;
    await using server = Server.start({
        environment: world,
        ...hosting,
        health: new Health("vault"),
        drainTimeout: 100,
        authenticate: async () => {
            const subject = principal.user.reference("universe", "alice");
            const now = world.clock.now();

            return {
                caller: Caller.of(subject),
                credential: {
                    kind: "session",
                    id: "alice",
                    audience: hosting.package.id,
                    scope: hosting.scope,
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                    assurance: { level, authenticatedAt: now },
                },
            };
        },
        router: implementation.router({
            rename: implementation.rename.handler(({ context }) => {
                // read the target the decision ran on
                if (context.target === undefined) {
                    throw new TypeError("a call requiring a permission has a target");
                }

                return context.target.id;
            }),
        }),
        access: {
            authorizer,
            database: storage.database,
            environment: world,
            target: async ({ input }) =>
                vault.reference(
                    hosting.scope,
                    schema.object({ id: schema.string() }).parse(input).id,
                ),
        },
    });
    const client = createClient(
        { router: procedures },
        {
            url: "https://vault.test",
            fetch: (request) => server.fetch(request),
        },
        world,
    );

    // challenge a single factor and rename after elevation
    await expect(client.rename({ id: "main" })).rejects.toMatchObject({
        code: "UNAUTHORIZED",
        status: 401,
        message: "authentication assurance is too low: authenticate again",
        data: RECENT,
    });
    level = 2;
    expect(await client.rename({ id: "main" })).toBe("main");
});
