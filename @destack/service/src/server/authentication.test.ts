import { copyScope } from "@destack/access/test";
import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import {
    accessTables,
    Authorizer,
    Policy,
    condition,
    relation,
    union,
    type AccessContext,
    type Subject,
    Authorization,
    principal,
} from "@destack/access";
import { boolean, defineTable, eq, text } from "@destack/db";
import { Condition } from "@destack/db/query";
import { TestDatabase } from "@destack/db/test";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import { Caller, TokenIssuer, TokenVerifier } from "../authentication/index.ts";
import { Health } from "../health/index.ts";
import { ServiceError } from "../error/index.ts";
import { defineProcedure, eventIterator } from "../service/index.ts";
import { createClient } from "../client/index.ts";
import { defineService } from "../declare/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./server.ts";
import type { ServiceContext } from "./context.ts";
import { createCaller, hosting } from "./tests/fixture.ts";

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
                new Caller({ ...createCaller("alice").authentication, scope: credentialScope }),
            router: implementation.router({
                read: implementation.read.handler(({ context }) => context.scope!),
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
            read: union(
                relation("owner"),
                relation("reader"),
                condition(Condition.eq("public", true)),
            ),
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
            new Caller({
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
    const test = await TestDatabase.create("sqlite", [noteTable, ...accessTables], {
        isMigrated: true,
    });
    onTestFinished(() => test.close());
    const database = test.database;
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
                ({ context }) => context.requireCaller().authentication.subject.id,
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
        headers: () => (bearer ? { authorization: `Bearer ${bearer}` } : {}),
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
        new Caller({
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
