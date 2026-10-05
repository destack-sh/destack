import { copyScope } from "@destack/access/test";
import { Scope, Subject } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import {
    accessTables,
    Authorizer,
    contained,
    Policy,
    resource,
    relation,
    union,
    type AccessContext,
    Authorization,
    principal,
} from "@destack/access";
import { boolean, defineTable, eq, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import {
    Authentication,
    Represented,
    TokenIssuer,
    TokenVerifier,
} from "../authentication/index.ts";
import { Health } from "../health/index.ts";
import { ServiceError } from "../error/index.ts";
import { defineProcedure, eventIterator } from "../service/index.ts";
import { createClient } from "../client/index.ts";
import { defineService } from "../declare/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./server.ts";
import type { ServiceContext } from "./context.ts";
import { createAuthentication, hosting } from "../test/fixture.ts";

test.each(["universe", "host-local", "account-personal", "space-personal"])(
    "enforce the configured %s authorization scope",
    async (scope) => {
        // serve one procedure in every scope
        const definition = {
            read: defineProcedure({ authentication: "identity", permission: null, audit: false })
                .route({ method: "GET", path: "/scope" })
                .output(schema.string()),
        };
        const implementation = implement(definition).$context<ServiceContext>();
        let credentialScope = scope;
        await using server = Server.start({
            ...hosting,
            scope,
            health: new Health("scope"),
            drainTimeout: 100,
            authenticate: async () =>
                new Authentication({
                    ...createAuthentication("alice").claims,
                    scope: credentialScope,
                }),
            router: implementation.router({
                read: implementation.read.handler(({ context }) => {
                    // read the scope the call runs in
                    if (context.scope === undefined) {
                        throw new TypeError("a scoped service call has a scope");
                    }

                    return context.scope;
                }),
            }),
        });
        const client = createClient(defineService("fixture", definition), {
            url: "https://fixture.test",
            fetch: (request) => server.fetch(request),
        });

        // return the scope and reject credentials of another scope
        expect(await client.read()).toBe(scope);
        credentialScope = "another-scope";
        await expect(client.read()).rejects.toMatchObject({
            code: "UNAUTHORIZED",
            message: "caller authentication is expired or has a different audience or scope",
        });
    },
);

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
    const service = {
        me: defineProcedure({ authentication: "identity", permission: null, audit: "access" })
            .route({ method: "GET", path: "/me" })
            .output(schema.string()),
        read: defineProcedure({
            authentication: "public",
            permission: note.permission("read"),
            audit: "access",
        })
            .route({ method: "GET", path: "/notes/{id}" })
            .input(key)
            .output(schema.string()),
        watch: defineProcedure({
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
        sign: (payload) =>
            new SignJWT(payload)
                .setProtectedHeader({ alg: "ES256", kid: "current" })
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
            new Authentication({
                subject,
                subjects: [subject],
                credential: { kind: "user", id: subject.id },
                audience: packageId,
                scope: spaceId,
                verifiedAt: now,
                expiresAt: now + 60000,
            }),
        );
        credentials.set(subject.id, token.accessToken);
    }

    // keep notes and their relationships in a database
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
    const storage = await TestDatabase.create("sqlite", [noteTable, ...accessTables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await copyScope(database, { packageId, type: "space", scope: "universe", id: spaceId });
    await database
        .insert(noteTable)
        .values(
            ["one", "two"].map((id) => ({ id, scope: spaceId, owner: owner.id, public: false })),
        );
    const asOwner: AccessContext = { subjects: [owner], now: Date.now(), attributes: {} };

    // keep the host's installation state
    let isEnabled = true;
    let invocations = 0;
    let authentications = 0;
    const outcomes: string[] = [];
    const resources = new ResourceContext();
    const implementation = implement(service).$context<ServiceContext>();
    await using server = Server.start({
        service: hosting.service,
        audience: packageId,
        scope: spaceId,
        resources,
        authenticate: async (request) => {
            authentications++;

            return request.headers.has("authorization") || request.headers.has("cookie")
                ? verifier.authenticate(request, spaceId)
                : null;
        },
        authorizeHost: async () => {
            if (!isEnabled) {
                throw new ServiceError("FORBIDDEN", { message: "host procedures are disabled" });
            }
        },
        router: implementation.router({
            me: implementation.me.handler(
                ({ context }) => context.requireAuthentication().claims.subject.id,
            ),
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
    const client = createClient(defineService("fixture", service), {
        url: "https://notes.example",
        headers: () =>
            bearer !== undefined && bearer !== "" ? { authorization: `Bearer ${bearer}` } : {},
        fetch: (request) =>
            server.fetch(
                transport === "direct"
                    ? request
                    : new Request(request, {
                          headers: new Headers([...request.headers, ["x-user-id", "owner"]]),
                      }),
            ),
    });
    expect(await client.me()).toBe("owner");
    expect(await client.read({ id: "one" })).toBe("personal note");
    expect(await client.read({ id: "one" })).toBe("personal note");
    expect(authentications).toBe(3);

    // restrict an owner's credential to one object
    const now = Date.now();
    const restricted = await issuer.issue(
        new Authentication({
            subject: owner,
            subjects: [owner],
            credential: { kind: "personal", id: "restricted-token" },
            audience: packageId,
            scope: spaceId,
            verifiedAt: now,
            expiresAt: now + 60000,
            permissions: [{ ...note.permission("read"), scope: spaceId, objectId: "one" }],
        }),
    );
    bearer = restricted.accessToken;
    expect(await client.read({ id: "one" })).toBe("personal note");
    await expect(client.read({ id: "two" })).rejects.toMatchObject({ code: "FORBIDDEN" });

    // grant a collaborator one object
    bearer = credentials.get(guest.id);
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "FORBIDDEN" });
    const shared = await new Authorization(authorizer, database, () => asOwner).grant({
        object: note.reference(spaceId, "one"),
        relation: "reader",
        subject: guest,
    });
    expect(await client.read({ id: "one" })).toBe("personal note");
    await expect(client.read({ id: "two" })).rejects.toMatchObject({ code: "FORBIDDEN" });
    await new Authorization(authorizer, database, () => asOwner).revoke(shared.object, shared.id);
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

test("decide a host's call as a space its cell represents, with the host as the actor, and refuse every other representation", async () => {
    // declare zones standing for spaces their cells represent, and notes the principals inside an account read
    const packageId = PackageId.parse("package-019f7480-0000-7000-8000-000000000003");
    const module = { id: packageId, name: "@example/zones", version: "2026.9.0" };
    const accountId = "account-019f7480-0000-7000-8000-000000000004";
    const zone = new Policy(module, {
        name: "zone",
        relations: {
            space: { subjects: [principal.space] },
            cell: { subjects: [principal.cell] },
        },
        permissions: { represent: union(relation("space"), relation("cell")) },
    });
    const account = new Policy(module, { name: "account", permissions: {}, scope: true });
    const note = new Policy(module, {
        name: "note",
        relations: {},
        permissions: { read: contained(principal.space) },
    });
    const zoneTable = defineTable("zone", {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        cell: text("cell").notNull(),
    });
    const noteTable = defineTable("note", {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
    });
    const authorizer = new Authorizer(
        [zone, account, note],
        [
            {
                policy: zone,
                table: zoneTable,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {
                    space: { column: "id", scope: Scope.universe.id },
                    cell: { column: "cell", scope: Scope.universe.id },
                },
            },
            {
                policy: note,
                table: noteTable,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: {},
            },
        ],
    );

    // keep a zone placing a space of the account on the first host's cell, and a note in the account
    const storage = await TestDatabase.create("sqlite", [zoneTable, noteTable, ...accessTables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const accountScope = account.reference(Scope.universe.id, accountId);
    await copyScope(database, accountScope);
    const spaceId = "space-019f7480-0000-7000-8000-000000000005";
    await database
        .insert(zoneTable)
        .values({ id: spaceId, scope: Scope.universe.id, cell: "host-a" });
    await database.insert(noteTable).values({ id: "one", scope: accountId });
    const space = principal.space.reference(Scope.universe.id, spaceId);

    // serve the caller's subject and actors and the note to hosts by their names
    const definition = {
        me: defineProcedure({ authentication: "identity", permission: null, audit: false })
            .route({ method: "GET", path: "/me" })
            .output(schema.array(schema.string())),
        read: defineProcedure({
            authentication: "identity",
            permission: note.permission("read"),
            audit: false,
        })
            .route({ method: "GET", path: "/note" })
            .output(schema.string()),
    };
    const implementation = implement(definition).$context<ServiceContext>();
    await using server = Server.start({
        service: hosting.service,
        audience: packageId,
        resources: new ResourceContext(),
        health: new Health("zones"),
        drainTimeout: 100,
        authorizeHost: async () => {},
        authenticate: async (request) => {
            // authenticate a named host of the account
            const name = request.headers.get("authorization");
            if (name === null) {
                return null;
            }
            const host = principal.host.reference(accountId, name);
            const now = Date.now();

            return new Authentication({
                subject: host,
                subjects: [host],
                credential: { kind: "host-key", id: name },
                audience: packageId,
                verifiedAt: now,
                expiresAt: now + 60000,
            });
        },
        router: implementation.router({
            me: implementation.me.handler(({ context }) => {
                // read the subject the call is decided for and each actor
                const { claims } = context.requireAuthentication();

                return [
                    Subject.key(claims.subject),
                    ...(claims.delegates ?? []).map(
                        (delegate) => `${Subject.key(delegate.subject)} ${delegate.authority}`,
                    ),
                ];
            }),
            read: implementation.read.handler(async () => "note"),
        }),
        access: {
            authorizer,
            database,
            target: async () => note.reference(accountId, "one"),
            standing: async (subject) =>
                Subject.same(subject, space)
                    ? {
                          permission: zone.permission("represent"),
                          object: zone.reference(Scope.universe.id, spaceId),
                          subject: space,
                          within: [accountScope],
                      }
                    : undefined,
        },
    });

    // call as a host, naming a principal it acts as or none
    const connect = (host: string | undefined, represented?: Subject) => {
        const fetch = (request: Request) => server.fetch(request);

        return createClient(defineService("fixture", definition), {
            url: "https://zones.example",
            headers: host === undefined ? {} : { authorization: host },
            fetch: represented === undefined ? fetch : Represented.fetch(fetch, represented),
        });
    };
    const other = principal.space.reference(
        Scope.universe.id,
        "space-019f7480-0000-7000-8000-000000000006",
    );

    // decide the serving host as the space and refuse the host itself, other cells and spaces, and anyone anonymous
    expect([
        await connect("host-a", space).me(),
        await connect("host-a", space).read(),
        await refusal(connect("host-a").read()),
        await refusal(connect("host-b", space).me()),
        await refusal(connect("host-a", other).me()),
        await refusal(connect(undefined, space).me()),
    ]).toEqual([
        [Subject.key(space), `${Subject.key(principal.host.reference(accountId, "host-a"))} full`],
        "note",
        ["FORBIDDEN", "permission denied: read"],
        ["FORBIDDEN", "permission denied: represent"],
        ["FORBIDDEN", `nothing here stands for ${Subject.key(other)}`],
        ["UNAUTHORIZED", "an anonymous caller represents no principal"],
    ]);
});
