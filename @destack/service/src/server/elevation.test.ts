import { copyScope } from "@destack/access/test";
import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { accessTables, Authorizer, Policy, principal, relation } from "@destack/access";
import { defineTable, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Caller } from "../authentication/index.ts";
import { Health } from "../health/index.ts";
import { defineProcedure } from "../service/index.ts";
import { createClient } from "../client/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./server.ts";
import type { ServiceContext } from "./context.ts";
import { hosting } from "./tests/fixture.ts";

/** The authentication renaming a vault requires. */
const RECENT = { assurance: 2, maxAge: 10 * 60 * 1000 };

test("challenge a hand-written procedure's caller for its elevated permission, then admit it once it authenticates again", async () => {
    // declare a vault its owner renames after recent strong authentication
    const module = {
        package: { id: hosting.audience, name: "@example/vault", version: "2026.9.0" },
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

    // hold alice's vault
    const storage = await TestDatabase.create("sqlite", [vaults, ...accessTables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await copyScope(storage.database, {
        packageId: hosting.audience,
        type: "space",
        scope: "universe",
        id: hosting.scope,
    });
    await storage.database
        .insert(vaults)
        .values({ id: "main", scope: hosting.scope, owner: "alice" });

    // serve renaming with the permission
    const service = {
        rename: defineProcedure({
            authentication: "identity",
            permission: vault.permission("rename"),
            audit: false,
        })
            .route({ method: "POST", path: "/vaults/{id}/rename" })
            .input(schema.object({ id: schema.string() }))
            .output(schema.string()),
    };
    const implementation = implement(service).$context<ServiceContext>();
    let level = 1;
    await using server = Server.start({
        ...hosting,
        health: new Health("vault"),
        drainTimeout: 100,
        authenticate: async () => {
            const subject = principal.user.reference("universe", "alice");
            const now = Date.now();

            return new Caller({
                subject,
                subjects: [subject],
                credential: { kind: "user", id: "alice" },
                audience: hosting.audience,
                scope: hosting.scope,
                verifiedAt: now,
                expiresAt: now + 60_000,
                assurance: { level, authenticatedAt: now },
            });
        },
        router: implementation.router({
            rename: implementation.rename.handler(({ context }) => context.target!.id),
        }),
        access: {
            authorizer,
            database: storage.database,
            target: async ({ input }) =>
                vault.reference(
                    hosting.scope,
                    schema.object({ id: schema.string() }).parse(input).id,
                ),
        },
    });
    const client = createClient(service, {
        url: "https://vault.test",
        fetch: (request) => server.fetch(request),
    });

    // challenge a single factor, then rename after elevation
    await expect(client.rename({ id: "main" })).rejects.toMatchObject({
        code: "INSUFFICIENT_AUTHENTICATION",
        status: 401,
        message: "authenticate again at the required assurance",
        data: RECENT,
    });
    level = 2;
    expect(await client.rename({ id: "main" })).toBe("main");
});
