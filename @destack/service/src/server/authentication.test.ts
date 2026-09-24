import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import {
    Access,
    AccessModel,
    AccessSnapshot,
    defineObject,
    relation,
    union,
    type Grant,
    type Subject,
} from "@destack/access";
import { exportJWK, generateKeyPair, SignJWT } from "jose";
import { Caller, TokenIssuer, TokenVerifier } from "../authentication/index.ts";
import { Health } from "../health/index.ts";
import { ServiceError } from "../error/index.ts";
import { defineProcedure, eventIterator } from "../service/index.ts";
import { createClient } from "../client/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./server.ts";
import type { ServiceContext } from "./context.ts";
import { createCaller, hosting } from "./tests/fixture.ts";

test.each(["global", "host-local", "account-personal", "space-personal"])(
    "enforce the configured %s authorization scope",
    async (scope) => {
        // use the same procedure and verified identity for every hosting scope
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
                read: implementation.read.handler(({ context }) => context.scope),
            }),
        });
        const client = createClient(definition, {
            url: "https://fixture.test",
            fetch: (request) => server.fetch(request),
        });

        // return the selected scope and reject credentials issued for another scope
        expect(await client.read()).toBe(scope);
        credentialScope = "another-scope";
        await expect(client.read()).rejects.toMatchObject({
            code: "UNAUTHORIZED",
            message: "caller authentication is expired or has a different audience or scope",
        });
    },
);

/** Apply the same identity and object rules to direct and forwarded user-service requests. */
test.each(["direct", "forwarded"])("host personal notes through %s requests", async (transport) => {
    // declare application permissions independently of hosting and identity providers
    const packageId = PackageId.parse("package-019f7480-0000-7000-8000-000000000001");
    const spaceId = "space-019f7480-0000-7000-8000-000000000002";
    const module = { package: { id: packageId, name: "@example/notes", version: "2026.9.0" } };
    const note = defineObject(
        {
            name: "note",
            attributes: {},
            relations: {
                owner: { kind: "subject", subjects: ["user"] },
                reader: { kind: "grant", subjects: ["user", "everyone"], permission: "share" },
            },
            permissions: {
                read: union(relation("owner"), relation("reader")),
                share: relation("owner"),
            },
        },
        module,
    );
    const model = new AccessModel([note]);
    const key = schema.object({ id: schema.string() });
    const service = {
        me: defineProcedure({ authentication: "identity", permission: null, audit: true })
            .route({ method: "GET", path: "/me" })
            .output(schema.string()),
        read: defineProcedure({
            authentication: "public",
            permission: note.permission("read"),
            audit: true,
        })
            .route({ method: "GET", path: "/notes/{id}" })
            .input(key)
            .output(schema.string()),
        watch: defineProcedure({
            authentication: "public",
            permission: note.permission("read"),
            audit: true,
        })
            .route({ method: "GET", path: "/notes/{id}/watch" })
            .input(key)
            .output(eventIterator(schema.string())),
    };

    // sign reusable service credentials with a real asymmetric key
    const keys = await generateKeyPair("ES256");
    const issuer = new TokenIssuer({
        issuer: "https://account.example",
        authority: { kind: "global" },
        sign: (payload) =>
            new SignJWT(payload)
                .setProtectedHeader({ alg: "ES256", kid: "current" })
                .sign(keys.privateKey),
    });
    const verifier = new TokenVerifier({
        issuer: "https://account.example",
        authority: { kind: "global" },
        audience: packageId,
        keys: { keys: [{ ...(await exportJWK(keys.publicKey)), kid: "current", alg: "ES256" }] },
    });
    const owner: Subject = { kind: "user", authority: "global", id: "owner" };
    const guest: Subject = { kind: "user", authority: "global", id: "guest" };
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

    // retain application records separately from the host's installation state
    let grants: Grant[] = [];
    let isEnabled = true;
    let invocations = 0;
    let authentications = 0;
    const outcomes: string[] = [];
    const resources = new ResourceContext();
    const implementation = implement(service).$context<ServiceContext>();
    await using server = Server.start({
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
                throw new ServiceError("FORBIDDEN");
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
        authorize: async ({ access, input, context }) => {
            if (access.permission) {
                const { id } = key.parse(input);
                const snapshot = new AccessSnapshot(
                    1,
                    [
                        {
                            reference: note.reference(spaceId, id),
                            attributes: {},
                            subjects: { owner: [owner] },
                            objects: {},
                        },
                    ],
                    grants,
                );
                const allowed = new Access(model, snapshot).check(
                    access.permission,
                    note.reference(spaceId, id),
                    context.access(),
                );
                if (!allowed) {
                    throw new ServiceError("FORBIDDEN");
                }
            }
        },
    });

    // forward the original credential without trusting identity headers from the relay
    let bearer: string | undefined = credentials.get(owner.id);
    const client = createClient(service, {
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

    // restrict an owner's credential to one object despite its broader application permissions
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

    // grant a collaborator access to one object and reject the same credential on another
    bearer = credentials.get(guest.id);
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "FORBIDDEN" });
    grants = [
        {
            id: "share-one",
            object: note.reference(spaceId, "one"),
            relation: "reader",
            subject: guest,
            createdAt: Date.now(),
            expiresAt: null,
            revokedAt: null,
        },
    ];
    expect(await client.read({ id: "one" })).toBe("personal note");
    await expect(client.read({ id: "two" })).rejects.toMatchObject({ code: "FORBIDDEN" });
    grants = [];
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "FORBIDDEN" });

    // permit public sharing without letting invalid credentials become anonymous
    grants = [
        {
            id: "public-one",
            object: note.reference(spaceId, "one"),
            relation: "reader",
            subject: { kind: "everyone" },
            createdAt: Date.now(),
            expiresAt: null,
            revokedAt: null,
        },
    ];
    bearer = undefined;
    expect(await client.read({ id: "one" })).toBe("personal note");
    await expect(client.me()).rejects.toMatchObject({ code: "UNAUTHORIZED" });
    bearer = "invalid";
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "UNAUTHORIZED" });

    // enforce host suspension on public requests and recheck grants during streams
    bearer = undefined;
    isEnabled = false;
    await expect(client.read({ id: "one" })).rejects.toMatchObject({ code: "FORBIDDEN" });
    isEnabled = true;
    const stream = await client.watch({ id: "one" });
    expect(await stream.next()).toEqual({ done: false, value: "first" });
    grants = [];
    await expect(stream.next()).rejects.toMatchObject({ code: "FORBIDDEN" });
    expect(invocations).toBe(5);
    expect(outcomes).toEqual([
        "started",
        "succeeded",
        "started",
        "succeeded",
        "started",
        "succeeded",
        "started",
        "succeeded",
        "started",
        "denied",
        "started",
        "denied",
        "started",
        "succeeded",
        "started",
        "denied",
        "started",
        "denied",
        "started",
        "succeeded",
        "started",
        "denied",
        "started",
        "denied",
        "started",
        "denied",
        "started",
        "denied",
    ]);
});
