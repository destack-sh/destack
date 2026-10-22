import { AccessFixture } from "@destack/access/test";
import { Scope, Subject } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Identifier, schema } from "@destack/schema";
import { PackageId, Package } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import {
    accessTables,
    Authorizer,
    Policy,
    resource,
    relation,
    union,
    AccessContext,
    type Authentication,
    Authorization,
    Caller,
    principal,
    Relationship,
} from "@destack/access";
import { boolean, defineTable, eq, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import { TokenIssuer, TokenVerifier } from "@destack/access/token";
import { Health } from "./health.ts";
import { CallError } from "../error/index.ts";
import { defineProcedure, eventIterator } from "../procedure/index.ts";
import { createClient } from "../client/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./http.ts";
import { RequestContext } from "./context.ts";
import { authenticateUser, fixturePackage, hosting } from "../test/fixture.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../index.ts";

test.each(["universe", "host-local", "account-personal", "space-personal"])(
    "enforce the configured %s authorization scope",
    async (scope) => {
        // serve one procedure in every scope
        const definition = {
            read: defineProcedure({
                package: Package.of(fixturePackage),
                authentication: "identity",
                permission: {
                    principals: [principal.user, principal.machine, principal.installation],
                },
                audit: false,
            })
                .route({ method: "GET", path: "/scope" })
                .output(schema.string()),
        };
        const implementation = implement(definition).$context<RequestContext>();
        let credentialScope = scope;
        await using server = Server.start({
            clock: Clock.system,
            ...hosting,
            scope,
            health: new Health("scope"),
            drainTimeout: 100,
            authenticate: async () => {
                const alice = authenticateUser("alice");

                return { ...alice, credential: { ...alice.credential, scope: credentialScope } };
            },
            router: implementation.router({
                read: implementation.read.handler(({ context }) => {
                    // read the scope the call runs in
                    if (context.scope === undefined) {
                        throw new TypeError("a scoped call has a scope");
                    }

                    return context.scope;
                }),
            }),
        });
        const client = createClient(
            { router: definition },
            {
                url: "https://fixture.test",
                fetch: (request) => server.fetch(request),
            },
        );

        // return the scope and reject credentials of another scope
        expect(await client.read()).toBe(scope);
        credentialScope = "another-scope";
        await expect(client.read()).rejects.toMatchObject({
            code: "UNAUTHORIZED",
            message: "caller authentication has a different audience or scope",
        });
    },
);

test("verify a credential the package verifies before the host, which answers every other request", async () => {
    // serve the caller's name, the package verifying its key and refusing a revoked one
    const definition = {
        me: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: { principals: [principal.user] },
            audit: false,
        })
            .route({ method: "GET", path: "/me" })
            .output(schema.string()),
    };
    const implementation = implement(definition).$context<RequestContext>();
    const verified: string[] = [];
    await using server = Server.start({
        clock: Clock.system,
        ...hosting,
        health: new Health("verify"),
        drainTimeout: 100,
        verify: async (request) => {
            const key = request.headers.get("package-key");
            verified.push(key ?? "none");
            if (key === "revoked") {
                throw new CallError("UNAUTHORIZED", { message: "the package key is revoked" });
            }

            return key === null ? undefined : authenticateUser("bob");
        },
        router: implementation.router({
            me: implementation.me.handler(({ context }) => Caller.sender(context.caller()).id),
        }),
    });
    const me = (headers: Record<string, string>) =>
        createClient(
            { router: definition },
            { url: "https://fixture.test", headers, fetch: (request) => server.fetch(request) },
        ).me();

    // answer the package's caller, the host's caller without the package's key, and refuse a revoked key
    expect({
        package: await me({ "package-key": "standing", authorization: "alice" }),
        host: await me({ authorization: "alice" }),
        revoked: await refusal(me({ "package-key": "revoked", authorization: "alice" })),
        verified,
    }).toEqual({
        package: "bob",
        host: "alice",
        revoked: ["UNAUTHORIZED", "the package key is revoked"],
        verified: ["standing", "none", "revoked"],
    });
});

/** Hand over a user verified for a package until a time. */
function handed(name: string, audience: PackageId, expiresAt: number): Authentication {
    const user = authenticateUser(name);

    return { ...user, credential: { ...user.credential, audience, expiresAt } };
}

test("serve a caller handed over in-process while it is current for this package, and refuse one verified for another package", async () => {
    // serve the caller's name to callers handed over by in-process code
    const definition = {
        me: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: { principals: [principal.user] },
            audit: false,
        })
            .route({ method: "GET", path: "/me" })
            .output(schema.string()),
    };
    const implementation = implement(definition).$context<RequestContext>();
    const other = PackageId.parse("package-019f7480-0000-7000-8000-000000000009");
    await using server = Server.start({
        clock: Clock.system,
        ...hosting,
        health: new Health("handed"),
        drainTimeout: 100,
        router: implementation.router({
            me: implementation.me.handler(({ context }) => Caller.sender(context.caller()).id),
        }),
    });
    const me = (authentication: Authentication) =>
        createClient(
            { router: definition },
            {
                url: "https://fixture.test",
                fetch: (request) => server.fetch(request, authentication),
            },
        ).me();

    // answer a current caller verified for this package, refusing an expired one and one verified for another
    const later = Date.now() + 60000;
    expect({
        current: await me(handed("alice", hosting.package.id, later)),
        expired: await refusal(me(handed("bob", hosting.package.id, Date.now() - 1))),
        other: await refusal(me(handed("carol", other, later))),
    }).toEqual({
        current: "alice",
        expired: ["UNAUTHORIZED", "caller authentication is not current"],
        other: ["UNAUTHORIZED", "caller authentication has a different audience or scope"],
    });
});

test("authorize a caller bound to a space in the scopes inside it, refusing the universe above it", async () => {
    // place a folder inside a space
    const packageId = hosting.package.id;
    const spaceId = "space-019f7480-0000-7000-8000-000000000003";
    const folderId = "folder-019f7480-0000-7000-8000-000000000004";
    const module = { package: { id: packageId, name: "@example/folders", version: "2026.9.0" } };
    const space = new Policy(module.package, { name: "space", permissions: {}, scope: true });
    const folder = new Policy(module.package, { name: "folder", permissions: {}, scope: true });
    const storage = await TestDatabase.create(accessTables, { isMigrated: true });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const fixture = new AccessFixture(database);
    await fixture.replicateScope({ packageId, type: "space", scope: "universe", id: spaceId });
    await fixture.replicateScope({ packageId, type: "folder", scope: spaceId, id: folderId });

    // call the space with a credential bound to it
    const alice = authenticateUser("alice");
    const credential = { ...alice.credential, audience: packageId, scope: spaceId };
    const context = new RequestContext(new Request("https://fixture.test/"), {
        clock: Clock.system,
        audience: packageId,
        scope: spaceId,
        authentication: { caller: alice.caller, credential },
        resources: new ResourceContext(),
        access: { authorizer: new Authorizer([space, folder], []), database },
    });
    const authorization = context.requireAuthorization();

    // resolve the space and its folder, refusing the universe, and access the folder as the caller
    expect([
        await refusal(authorization.in(spaceId)),
        await refusal(authorization.in(folderId)),
        await refusal(authorization.in("universe")),
        context.access(folderId).subject,
    ]).toEqual([
        "done",
        "done",
        ["UNAUTHORIZED", `the caller's credential applies to ${spaceId} only`],
        alice.caller.subject,
    ]);
});

/** Apply the same identity and object rules to direct and forwarded requests. */
test.each(["direct", "forwarded"])("host personal notes through %s requests", async (transport) => {
    // declare the note permissions
    const packageId = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");
    const spaceId = "space-019f7480-0000-7000-8000-000000000002";
    const module = { package: { id: packageId, name: "@example/notes", version: "2026.9.0" } };
    const note = new Policy(module.package, {
        name: "note",
        attributes: { public: "boolean" },
        relations: {
            owner: { subjects: [principal.user] },
            reader: { subjects: [principal.user], grantedBy: "share" },
        },
        permissions: {
            read: union(relation("owner"), relation("reader"), resource({ public: true })),
            share: relation("owner"),
        },
    });
    const key = schema.object({ id: schema.string() });
    const procedures = {
        me: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: { principals: [principal.user, principal.machine, principal.installation] },
            audit: "access",
        })
            .route({ method: "GET", path: "/me" })
            .output(schema.string()),
        read: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: note.permission("read"),
            audit: "access",
        })
            .route({ method: "GET", path: "/notes/{id}" })
            .input(key)
            .output(schema.string()),
        watch: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: note.permission("read"),
            audit: "access",
        })
            .route({ method: "GET", path: "/notes/{id}/watch" })
            .input(key)
            .output(eventIterator(schema.string())),
    };

    // sign credentials with an asymmetric key
    const keys = await generateKeyPair("ES256");
    const issuer = new TokenIssuer({
        issuer: "https://account.example",
        authority: { kind: "universe" },
        sign: (payload, header) =>
            new SignJWT(payload)
                .setProtectedHeader({ alg: "ES256", kid: "current", ...header })
                .sign(keys.privateKey),
    });
    const verifier = new TokenVerifier({
        issuer: "https://account.example",
        authority: { kind: "universe" },
        audience: packageId,
        keys: { keys: [{ ...(await exportJWK(keys.publicKey)), kid: "current", alg: "ES256" }] },
    });
    const owner: Subject = principal.user.reference("universe", "owner");
    const guest: Subject = principal.user.reference("universe", "guest");
    const credentials = new Map<string, string>();
    for (const subject of [owner, guest]) {
        const now = Date.now();
        const token = await issuer.issue(
            {
                caller: Caller.of(subject),
                credential: {
                    kind: "session",
                    id: subject.id,
                    client: Identifier.create("client"),
                    audience: packageId,
                    scope: spaceId,
                    verifiedAt: now,
                    expiresAt: now + 60000,
                },
            },
            Date.now(),
        );
        credentials.set(subject.id, token.accessToken);
    }

    // store notes and their relationships in a database
    const noteTable = defineTable("note", {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        owner: text("owner").notNull(),
        public: boolean("public").notNull(),
    });
    const space = new Policy(module.package, { name: "space", permissions: {}, scope: true });
    const authorizer = new Authorizer(
        [note, space],
        [
            {
                policy: note,
                table: noteTable,
                id: "id",
                scope: "scope",
                attributes: { public: "public" },
                relations: { owner: { column: "owner", scope: "universe" } },
            },
        ],
    );
    const storage = await TestDatabase.create(
        tablesOf({ packages: [], follows: [], tables: [noteTable] }).tables,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;
    await new AccessFixture(database).replicateScope({
        packageId,
        type: "space",
        scope: "universe",
        id: spaceId,
    });
    await database
        .insert(noteTable)
        .values(
            ["one", "two"].map((id) => ({ id, scope: spaceId, owner: owner.id, public: false })),
        );
    const asOwner = AccessContext.of(Caller.of(owner), Date.now());

    // store the host's installation state
    let isEnabled = true;
    let invocations = 0;
    let authentications = 0;
    const outcomes: string[] = [];
    const resources = new ResourceContext();
    const implementation = implement(procedures).$context<RequestContext>();
    await using server = Server.start({
        clock: Clock.system,
        package: hosting.package,
        scope: spaceId,
        resources,
        authenticate: async (request) => {
            authentications++;

            return request.headers.has("authorization") || request.headers.has("cookie")
                ? verifier.authenticate(request, Date.now(), spaceId)
                : undefined;
        },
        authorizeMachine: async () => {
            if (!isEnabled) {
                throw new CallError("FORBIDDEN", { message: "host procedures are disabled" });
            }
        },
        router: implementation.router({
            me: implementation.me.handler(({ context }) => Caller.sender(context.caller()).id),
            read: implementation.read.handler(({ context }) => {
                invocations++;
                expect(context.resources).toBe(resources);

                return "personal note";
            }),
            watch: implementation.watch.handler(async function* () {
                yield "first";
                yield "second";
            }),
        }),
        health: new Health("notes"),
        drainTimeout: 1000,
        audit: async (event) => {
            outcomes.push(event.outcome);
        },
        access: {
            authorizer,
            database,
            target: async ({ input }) => note.reference(spaceId, key.parse(input).id),
        },
    });

    // forward the original credential
    let bearer: string | undefined = credentials.get(owner.id);
    const client = createClient(
        { router: procedures },
        {
            url: "https://notes.example",
            headers: () =>
                bearer !== undefined && bearer !== "" ? { authorization: `Bearer ${bearer}` } : {},
            fetch: (request) =>
                server.fetch(
                    transport === "direct"
                        ? request
                        : new Request(request, {
                              headers: new Headers([...request.headers, ["Destack-User", "owner"]]),
                          }),
                ),
        },
    );
    expect(await client.me()).toBe("owner");
    expect(await client.read({ id: "one" })).toBe("personal note");
    expect(await client.read({ id: "one" })).toBe("personal note");
    expect(authentications).toBe(3);

    // limit an owner's token to one object by their stored grant to it
    const now = Date.now();
    const token = principal.client.reference(Scope.universe.id, "client-restricted");
    const grantId = await new AccessFixture(database).replicateGrant(owner, token, [
        { ...note.permission("read"), scope: spaceId, objectId: "one" },
    ]);
    const restricted = await issuer.issue(
        {
            caller: {
                ...Caller.of(owner),
                actors: [token],
                grants: [{ id: grantId, scope: owner.id }],
            },
            credential: {
                kind: "personal",
                id: "restricted-token",
                client: Identifier.create("client"),
                audience: packageId,
                scope: spaceId,
                verifiedAt: now,
                expiresAt: now + 60000,
            },
        },
        Date.now(),
    );
    bearer = restricted.accessToken;
    expect(await client.read({ id: "one" })).toBe("personal note");
    await expect(client.read({ id: "two" })).rejects.toMatchObject({ code: "FORBIDDEN" });

    // grant a collaborator one object
    bearer = credentials.get(guest.id);
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "FORBIDDEN" });
    const byOwner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(authorizer, database, () => asOwner, call);
    const shared = await byOwner((authorization) =>
        Relationship.grant(
            {
                object: note.reference(spaceId, "one"),
                relation: "reader",
                subject: guest,
            },
            authorization,
        ),
    );
    expect(await client.read({ id: "one" })).toBe("personal note");
    await expect(client.read({ id: "two" })).rejects.toMatchObject({ code: "FORBIDDEN" });
    await byOwner((authorization) => Relationship.revoke(shared.object, shared.id, authorization));
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "FORBIDDEN" });

    // allow public notes and reject invalid credentials
    await database.update(noteTable).set({ public: true }).where(eq(noteTable.id, "one"));
    bearer = undefined;
    expect(await client.read({ id: "one" })).toBe("personal note");
    await expect(client.me()).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    bearer = "invalid";
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "UNAUTHORIZED" });

    // enforce suspension and recheck grants during streams
    bearer = undefined;
    isEnabled = false;
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "FORBIDDEN" });
    isEnabled = true;
    const stream = await client.watch({ id: "one" });
    expect(await stream.next()).toEqual({ done: false, value: "first" });
    await database.update(noteTable).set({ public: false }).where(eq(noteTable.id, "one"));
    await expect(stream.next()).rejects.toMatchObject({ code: "FORBIDDEN" });
    expect(invocations).toBe(5);
    expect(outcomes).toEqual([
        "success",
        "success",
        "success",
        "success",
        "denied",
        "denied",
        "success",
        "denied",
        "denied",
        "success",
        "denied",
        "denied",
        "denied",
        "denied",
    ]);
});
