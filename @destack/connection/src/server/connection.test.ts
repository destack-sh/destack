import type { CallInput } from "@destack/object";
import { nth, present } from "@destack/schema";
import { RequestId, type ObjectReference, Subject } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { integration, authorizationRequest, connection } from "../object/index.ts";
import { ConnectionFixture, RESOURCE, testIssuer } from "../test/index.ts";
import { Runner } from "./token.ts";

/** The redirect URI the test integration registered. */
const REDIRECT = "https://app.test/callback";

/** Serve connections for one test. */
async function open(): Promise<ConnectionFixture> {
    const fixture = await ConnectionFixture.open();
    onTestFinished(() => fixture.close());

    return fixture;
}

/** Create a connection as the owner, answering its reference. */
async function create(
    fixture: ConnectionFixture,
    input: Omit<CallInput<typeof connection, "create">, "scope" | "vault">,
): Promise<ObjectReference> {
    const created = await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("create", {
                    scope: fixture.scope.id,
                    vault: fixture.vaultReference,
                    ...input,
                }),
            ],
        })
        .then(([written]) => written);

    return connection.reference(fixture.scope.id, created.id);
}

/** A space other than the fixture's. */
const OTHER_SCOPE = "space-019f5530-8000-7000-8000-0000000000c1";

/** A connection the fixture's caller does not manage. */
const OTHER_CONNECTION = "connection-019f5530-8000-7000-8000-0000000000c2";

/** Read a connection's Authorized condition and expiry as status, reason and whether it expires. */
async function authorized(fixture: ConnectionFixture, reference: ObjectReference) {
    const row = await readConnection(fixture, reference);
    const condition = row.conditions["Authorized"];

    return [condition?.status, condition?.reason, row.expiresAt !== null];
}

/** Read a connection's row. */
async function readConnection(fixture: ConnectionFixture, reference: ObjectReference) {
    return nth(
        (await fixture.connections()).filter((each) => each.id === reference.id),
        0,
    );
}

/** Lend a connection to a request to the external API, answering its Authorization header. */
async function lease(
    fixture: ConnectionFixture,
    reference: ObjectReference,
): Promise<string | null> {
    const leased = await fixture.bindAndLease(reference, new Request("https://api.test/items"));

    return leased.request.headers.get("authorization");
}

/** Start authorizing a connection as the owner in a session, redirected to the test URI. */
function authorize(fixture: ConnectionFixture, reference: ObjectReference, session?: string) {
    return fixture.objects
        .mutate(fixture.context(session).access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("authorize", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    redirectUri: REDIRECT,
                }),
            ],
        })
        .then(([written]) => written);
}

/** Complete a connection's authorization as the owner in a session with a redirect's parameters. */
function completeAuthorization(
    fixture: ConnectionFixture,
    reference: ObjectReference,
    parameters: Record<string, string>,
    session?: string,
) {
    return fixture.objects
        .mutate(fixture.context(session).access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("complete", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    parameters,
                }),
            ],
        })
        .then(([written]) => written);
}

test("authorize a person's account with a code and PKCE in their session, once, checking the issuer, and lease its token", async () => {
    const fixture = await open();
    const registered = await fixture.staticIntegration();
    const reference = await create(fixture, {
        protocol: { kind: "authorization-code" },
        integration: registered,
        scopes: ["items:read"],
    });
    await fixture.control();

    // start the authorization and approve it at the server
    const { url } = await authorize(fixture, reference);
    const page = new URL(url);
    const redirect = await fixture.server.approve(url);
    const complete = (parameters: Record<string, string>, session?: string) =>
        refusal(completeAuthorization(fixture, reference, parameters, session));

    // refuse another session and issuer and a forged state before completing it once
    const refusals = [
        await complete(redirect, "session-two"),
        await complete({ ...redirect, iss: "https://evil.test" }),
        await complete({ ...redirect, state: "forged" }),
    ];
    const pending = await authorized(fixture, reference);
    const completed = await complete(redirect);
    const replayed = await complete(redirect);

    expect({
        page: [
            page.origin + page.pathname,
            page.searchParams.get("code_challenge_method"),
            page.searchParams.get("scope"),
        ],
        refusals,
        pending,
        completed,
        replayed,
        authorized: await authorized(fixture, reference),
        leased: await lease(fixture, reference),
    }).toEqual({
        page: ["https://auth.test/authorize", "S256", "items:read"],
        refusals: [
            [
                "INVALID_AUTHORIZATION",
                "only the session that started an authorization completes it",
            ],
            [
                "INVALID_AUTHORIZATION",
                "the authorization response returns issuer https://evil.test instead of https://auth.test",
            ],
            ["INVALID_AUTHORIZATION", "no pending authorization has this state"],
        ],
        pending: ["false", "Pending", false],
        completed: "done",
        replayed: ["INVALID_AUTHORIZATION", "no pending authorization has this state"],
        authorized: ["true", "Authorized", true],
        leased: "Bearer access-1",
    });
});

test("acquire a client credentials token in the controller, refresh it before expiry there, and lease only current tokens", async () => {
    const fixture = await open();
    const registered = await fixture.staticIntegration();
    const reference = await create(fixture, {
        protocol: { kind: "client-credentials" },
        integration: registered,
    });

    // acquire the token and lease it
    const waits = await fixture.control();
    const first = await lease(fixture, reference);

    // lease a token near its expiry as it is, then refresh it once in the controller
    await moveExpiry(fixture, Date.now() + 1000);
    const near = await Promise.all([1, 2, 3].map(() => lease(fixture, reference)));
    await fixture.control();
    const refreshed = await lease(fixture, reference);

    // refuse a lapsed token the controller has yet to refresh
    await moveExpiry(fixture, Date.now() - 1000);
    const lapsed = await refusal(lease(fixture, reference));
    const tokenRequests = fixture.server.requests.filter(
        (request) => request.path === "https://auth.test/token",
    );

    expect({
        waited: (waits[0] ?? 0) > 3_000_000,
        first,
        near,
        refreshed,
        lapsed,
        tokenRequests: tokenRequests.map((request) => request.body["grant_type"]),
    }).toEqual({
        waited: true,
        first: "Bearer access-1",
        near: ["Bearer access-1", "Bearer access-1", "Bearer access-1"],
        refreshed: "Bearer access-2",
        lapsed: [
            "SERVICE_UNAVAILABLE",
            `connection ${reference.id} has a lapsed token, which its controller refreshes`,
        ],
        tokenRequests: ["client_credentials", "client_credentials"],
    });
});

/** Move the fixture's one connection's token expiry, in its row and its vault. */
async function moveExpiry(fixture: ConnectionFixture, expiresAt: number): Promise<void> {
    await fixture.database.update(connection.table).set({ expiresAt });
    const row = present((await fixture.connections())[0], "the connection");
    const runner = Runner.of(fixture.objects);
    const credential = await fixture.tokens.read(runner, row);
    await fixture.tokens.write(runner, row, {
        ...credential,
        token: { ...present(credential.token, "the token"), expiresAt },
    });
}

test("register a client dynamically at a protected resource's authorization server and authorize through it", async () => {
    const fixture = await open();
    const reference = await create(fixture, {
        protocol: { kind: "authorization-code" },
        resource: RESOURCE,
    });

    // discover the server, register a client and authorize with the resource indicator
    const { url } = await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("authorize", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    redirectUri: REDIRECT,
                }),
            ],
        })
        .then(([written]) => written);
    await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("complete", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    parameters: await fixture.server.approve(url),
                }),
            ],
        })
        .then(([written]) => written);
    const [row] = await fixture.connections();

    expect({
        resource: new URL(url).searchParams.get("resource"),
        isRegistered: row?.integration !== null,
        paths: fixture.server.requests.map((request) => request.path),
        leased: await lease(fixture, reference),
    }).toEqual({
        resource: RESOURCE,
        isRegistered: true,
        paths: [
            "https://mcp.test/.well-known/oauth-protected-resource/mcp",
            "https://auth.test/.well-known/oauth-authorization-server",
            "https://auth.test/register",
            "https://auth.test/token",
        ],
        leased: "Bearer access-1",
    });
});

test("poll a device authorization in the controller until the person approves it", async () => {
    const fixture = await open();
    const registered = await fixture.staticIntegration();
    const reference = await create(fixture, {
        protocol: { kind: "device-code" },
        integration: registered,
    });
    const { url, userCode } = await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [connection.call("authorize", { scope: fixture.scope.id, id: reference.id })],
        })
        .then(([written]) => written);

    // poll while pending and after approval
    const waiting = await fixture.control();
    fixture.server.approveDevice(present(userCode, "the user code"));
    await fixture.control();

    expect({
        url,
        userCode,
        waiting,
        authorized: await authorized(fixture, reference),
        leased: await lease(fixture, reference),
    }).toEqual({
        url: "https://auth.test/activate",
        userCode: "USER-1",
        waiting: [1000],
        authorized: ["true", "Authorized", true],
        leased: "Bearer access-1",
    });
});

test("revoke a connection's refresh token at its server and destroy its credential", async () => {
    const fixture = await open();
    const registered = await fixture.staticIntegration();
    const reference = await create(fixture, {
        protocol: { kind: "authorization-code" },
        integration: registered,
    });
    const { url } = await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("authorize", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    redirectUri: REDIRECT,
                }),
            ],
        })
        .then(([written]) => written);
    await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("complete", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    parameters: await fixture.server.approve(url),
                }),
            ],
        })
        .then(([written]) => written);

    // revoke it and refuse to lease it
    await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [connection.call("revoke", { scope: fixture.scope.id, id: reference.id })],
        })
        .then(([written]) => written);

    expect({
        revoked: fixture.server.revoked,
        authorized: await authorized(fixture, reference),
        credential: await refusal(
            fixture.tokens.read(
                Runner.of(fixture.objects),
                await readConnection(fixture, reference),
            ),
        ),
        leased: await refusal(lease(fixture, reference)),
    }).toEqual({
        revoked: ["refresh-1"],
        authorized: ["false", "Revoked", false],
        credential: ["UNAUTHORIZED", `connection ${reference.id} has no credential`],
        leased: ["PRECONDITION_FAILED", `connection ${reference.id} is unauthorized`],
    });
});

test("install an app through the installation issuing its tokens, lease an installation token, and uninstall it on revocation", async () => {
    // serve an issuer that mints a token of each installation it reads from the setup redirect
    const calls: string[] = [];
    const called = (method: string, installation: string, caller: { subject?: Subject }) =>
        calls.push(`${method} ${installation} by ${caller.subject?.id}`);
    const fixture = await ConnectionFixture.open({
        issuer: testIssuer.implement({
            methods: {
                issue: {
                    prepare: async (context) => {
                        called("issue", context.input.installation, context.caller);

                        return { accessToken: "ghs_installation", tokenType: "token" };
                    },
                    handle: async (context) => context.prepared,
                },
                uninstall: {
                    prepare: async (context) => {
                        called("uninstall", context.input.installation, context.caller);

                        return {};
                    },
                    handle: async (context) => context.prepared,
                },
                installation: async (call) => ({
                    installation: present(
                        call.input.parameters["installation_id"],
                        "the installation",
                    ),
                }),
            },
        }),
    });
    onTestFinished(() => fixture.close());

    // register the app as issued by the installation, refusing a key beside it
    const app = {
        scope: fixture.scope.id,
        issuer: await fixture.createIssuer(),
        endpoints: {
            installation: "https://github.test/apps/destack-test/installations/new",
            isIssuerReturned: false,
        },
        clientId: "Iv1.app",
        authentication: "none" as const,
        redirectUris: ["https://app.test/setup"],
    };
    const register = (input: CallInput<typeof integration, "create">) =>
        fixture.objects
            .mutate(fixture.context().access(), {
                id: RequestId.create(Date.now()),
                calls: [integration.call("create", input)],
            })
            .then(([written]) => written);
    const keyed = await refusal(
        register({
            ...app,
            vault: fixture.vaultReference,
            secret: "-----BEGIN RSA PRIVATE KEY-----",
        }),
    );
    const created = await register(app);
    const reference = await create(fixture, {
        protocol: { kind: "installation" },
        integration: integration.reference(fixture.scope.id, created.id),
    });

    // install it from the installation page, completing with the installation the redirect returns
    const { url } = await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("authorize", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    redirectUri: "https://app.test/setup",
                }),
            ],
        })
        .then(([written]) => written);
    const state = present(new URL(url).searchParams.get("state"), "the state");
    await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("complete", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    parameters: { installation_id: "4242", setup_action: "install", state },
                }),
            ],
        })
        .then(([written]) => written);

    // obtain the installation token in the controller, lease it, then revoke the connection
    await fixture.control();
    const leased = await lease(fixture, reference);
    await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [connection.call("revoke", { scope: fixture.scope.id, id: reference.id })],
        })
        .then(([written]) => written);
    const row = await readConnection(fixture, reference);

    // ask the issuer as the connection installation for the token and the uninstallation
    const installation = fixture.tokens.options.consumer.id;
    expect({
        keyed,
        page: `${new URL(url).origin}${new URL(url).pathname}`,
        subject: row.subject,
        leased,
        calls,
    }).toEqual({
        keyed: [
            "BAD_REQUEST",
            "an installed app's issuer holds its key, so its integration has no secret",
        ],
        page: "https://github.test/apps/destack-test/installations/new",
        subject: "4242",
        leased: "token ghs_installation",
        calls: [`issue 4242 by ${installation}`, `uninstall 4242 by ${installation}`],
    });
});

test("send a key connection's key in its header, never writing it to the rows or the journal", async () => {
    const fixture = await open();
    const key = "sk-test-0123456789";
    const reference = await create(fixture, {
        protocol: { kind: "key", header: "x-api-key" },
        material: { kind: "key", key },
    });
    await fixture.control();
    const leased = await fixture.bindAndLease(reference, new Request("https://api.test/items"));

    // list the tables whose rows contain the key
    const leaking: string[] = [];
    for (const [name, rows] of Object.entries(await fixture.dump())) {
        if (JSON.stringify(rows).includes(key)) {
            leaking.push(name);
        }
    }

    expect({
        header: leased.request.headers.get("x-api-key"),
        authorized: await authorized(fixture, reference),
        leaking,
    }).toEqual({
        header: key,
        authorized: ["true", "Supplied", false],
        leaking: [],
    });
});

test("refresh a person's token with its rotating refresh token, and mark the connection expired once the server refuses the grant", async () => {
    const fixture = await open();
    const registered = await fixture.staticIntegration();
    const reference = await authorizeCode(fixture, registered);

    // expire the token and let the controller refresh it with the refresh token
    const expire = async (refreshToken: string) => {
        const credential = await fixture.tokens.read(
            Runner.of(fixture.objects),
            await readConnection(fixture, reference),
        );
        const token = present(credential.token, "the token");
        await fixture.tokens.write(
            Runner.of(fixture.objects),
            { id: reference.id, vault: fixture.vaultReference },
            { ...credential, token: { ...token, refreshToken, expiresAt: Date.now() } },
        );
        await fixture.database.update(connection.table).set({ expiresAt: Date.now() });
    };
    await expire("refresh-1");
    await fixture.control();
    const refreshed = [await lease(fixture, reference), await authorized(fixture, reference)];

    // present the used refresh token again for the server to refuse
    await expire("refresh-1");
    await fixture.control();

    expect({ refreshed, expired: await authorized(fixture, reference) }).toEqual({
        refreshed: ["Bearer access-2", ["true", "Authorized", true]],
        expired: ["false", "Expired", false],
    });
});

test("answer no credential from any procedure: connections, integrations and authorizations read without secrets", async () => {
    const fixture = await open();
    const registered = await fixture.staticIntegration();
    const reference = await authorizeCode(fixture, registered);
    await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("authorize", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    redirectUri: REDIRECT,
                }),
            ],
        })
        .then(([written]) => written);

    // read every object through the package's reading procedures
    const read = { scope: fixture.scope.id };
    const answers = JSON.stringify([
        await fixture.objects.query(fixture.context().access(), connection.call("list", read)),
        await fixture.objects.query(
            fixture.context().access(),
            connection.call("get", { ...read, id: reference.id }),
        ),
        await fixture.objects.query(fixture.context().access(), integration.call("list", read)),
        await fixture.objects.query(
            fixture.context().access(),
            authorizationRequest.call("list", read),
        ),
    ]);

    // list the tokens, client secret, pending verifier and session an answer has
    const pending = nth(await fixture.database.select().from(authorizationRequest.table), 0);
    const credentials = [
        "access-1",
        "refresh-1",
        fixture.server.clientSecret,
        present(pending.verifier, "the pending verifier"),
        pending.session,
    ];

    expect(credentials.filter((credential) => answers.includes(credential))).toEqual([]);
});

/** Authorize a code connection of an integration through the test server, returning it. */
async function authorizeCode(
    fixture: ConnectionFixture,
    registered: ObjectReference,
): Promise<ObjectReference> {
    const reference = await create(fixture, {
        protocol: { kind: "authorization-code" },
        integration: registered,
    });
    const { url } = await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("authorize", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    redirectUri: REDIRECT,
                }),
            ],
        })
        .then(([written]) => written);
    await fixture.objects
        .mutate(fixture.context().access(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("complete", {
                    scope: fixture.scope.id,
                    id: reference.id,
                    parameters: await fixture.server.approve(url),
                }),
            ],
        })
        .then(([written]) => written);

    return reference;
}

test("refuse a connection lacking the client its kind authorizes through", async () => {
    const fixture = await open();
    const refused = (input: Parameters<typeof create>[1]) => refusal(create(fixture, input));

    // create an authorization code connection without an integration or a resource, and a client credentials one without an integration
    expect([
        await refused({ protocol: { kind: "authorization-code" } }),
        await refused({ protocol: { kind: "client-credentials" } }),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "an authorization-code connection needs an integration, or a resource whose server registers its client",
        ],
        ["BAD_REQUEST", "a client-credentials connection needs its integration"],
    ]);
});

test("exchange the token of a connection its creator manages, refusing another scope's or one it does not manage", async () => {
    const fixture = await open();
    const registered = await fixture.staticIntegration();
    const exchange = (source: ObjectReference) =>
        refusal(
            create(fixture, {
                protocol: { kind: "token-exchange", source },
                integration: registered,
            }),
        );

    // name another scope's connection, then one of this scope the caller does not manage
    const elsewhere = connection.reference(OTHER_SCOPE, OTHER_CONNECTION);
    const unmanaged = connection.reference(fixture.scope.id, OTHER_CONNECTION);
    expect([await exchange(elsewhere), await exchange(unmanaged)]).toEqual([
        ["FORBIDDEN", `a token exchange's source must be a connection of ${fixture.scope.id}`],
        ["FORBIDDEN", "permission denied: manage"],
    ]);

    // exchange the token of a connection the caller manages
    const own = await create(fixture, {
        protocol: { kind: "authorization-code" },
        integration: registered,
    });
    const exchanged = await create(fixture, {
        protocol: { kind: "token-exchange", source: own },
        integration: registered,
    });
    expect((await readConnection(fixture, exchanged)).protocol).toEqual({
        kind: "token-exchange",
        source: own,
    });
});
