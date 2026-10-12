import type { ObjectCallInput } from "@destack/object";
import { aligned, present } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import type { ObjectReference } from "@destack/sync";
import { integration, authorizationRequest, connection } from "../object/index.ts";
import { ConnectionFixture, RESOURCE } from "../test/index.ts";

/** The redirect URI the test integration registered. */
const REDIRECT = "https://app.test/callback";

/** Open a connection service for one test. */
async function open(): Promise<ConnectionFixture> {
    const fixture = await ConnectionFixture.open();
    onTestFinished(() => fixture.close());

    return fixture;
}

/** Create a connection as the owner, answering its reference. */
async function create(
    fixture: ConnectionFixture,
    input: Omit<ObjectCallInput<typeof connection, "create">, "scope" | "vault">,
): Promise<ObjectReference> {
    const created = await fixture.objects
        .mutate(fixture.context().caller(), {
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

/** Read a connection's Authorized condition and expiry as status, reason and whether it expires. */
async function authorized(fixture: ConnectionFixture, reference: ObjectReference) {
    const row = aligned(
        (await fixture.connections()).filter((each) => each.id === reference.id),
        0,
    );
    const condition = row.conditions["Authorized"];

    return [condition?.status, condition?.reason, row.expiresAt !== null];
}

/** Lend a connection to a request to the external API, answering its Authorization header. */
async function lend(
    fixture: ConnectionFixture,
    reference: ObjectReference,
): Promise<string | null> {
    const loan = await fixture.lender.lend(
        reference,
        new Request("https://api.test/items"),
        Date.now(),
    );

    return loan.request.headers.get("authorization");
}

/** Start authorizing a connection as the owner in a session, redirected to the test URI. */
function authorize(fixture: ConnectionFixture, reference: ObjectReference, session?: string) {
    return fixture.objects
        .mutate(fixture.context(session).caller(), {
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
        .mutate(fixture.context(session).caller(), {
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

test("authorize a person's account with a code and PKCE in their session, once, checking the issuer, and lend its token", async () => {
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
        lent: await lend(fixture, reference),
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
        lent: "Bearer access-1",
    });
});

test("acquire a client credentials token in the controller and refresh it before expiry once for concurrent lends", async () => {
    const fixture = await open();
    const registered = await fixture.staticIntegration();
    const reference = await create(fixture, {
        protocol: { kind: "client-credentials" },
        integration: registered,
    });

    // acquire the token and lend it
    const waits = await fixture.control();
    const first = await lend(fixture, reference);

    // let the token near its expiry and lend it three times at once
    await fixture.database.update(connection.table).set({ expiresAt: Date.now() + 1000 });
    const [row] = await fixture.connections();
    const credential = await fixture.tokens.read(fixture.objects, present(row, "the connection"));
    await fixture.tokens.write(fixture.objects, present(row, "the connection"), {
        ...credential,
        token: { ...present(credential.token, "the token"), expiresAt: Date.now() + 1000 },
    });
    const concurrent = await Promise.all([1, 2, 3].map(() => lend(fixture, reference)));
    const tokenRequests = fixture.server.requests.filter(
        (request) => request.path === "https://auth.test/token",
    );

    expect({
        waited: (waits[0] ?? 0) > 3_000_000,
        first,
        concurrent,
        tokenRequests: tokenRequests.map((request) => request.body["grant_type"]),
    }).toEqual({
        waited: true,
        first: "Bearer access-1",
        concurrent: ["Bearer access-2", "Bearer access-2", "Bearer access-2"],
        tokenRequests: ["client_credentials", "client_credentials"],
    });
});

test("register a client dynamically at a protected resource's authorization server and authorize through it", async () => {
    const fixture = await open();
    const reference = await create(fixture, {
        protocol: { kind: "authorization-code" },
        resource: RESOURCE,
    });

    // discover the server, register a client and authorize with the resource indicator
    const { url } = await fixture.objects
        .mutate(fixture.context().caller(), {
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
        .mutate(fixture.context().caller(), {
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
        lent: await lend(fixture, reference),
    }).toEqual({
        resource: RESOURCE,
        isRegistered: true,
        paths: [
            "https://mcp.test/.well-known/oauth-protected-resource/mcp",
            "https://auth.test/.well-known/oauth-authorization-server",
            "https://auth.test/register",
            "https://auth.test/token",
        ],
        lent: "Bearer access-1",
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
        .mutate(fixture.context().caller(), {
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
        lent: await lend(fixture, reference),
    }).toEqual({
        url: "https://auth.test/activate",
        userCode: "USER-1",
        waiting: [1000],
        authorized: ["true", "Authorized", true],
        lent: "Bearer access-1",
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
        .mutate(fixture.context().caller(), {
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
        .mutate(fixture.context().caller(), {
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

    // revoke it and refuse to lend it
    await fixture.objects
        .mutate(fixture.context().caller(), {
            id: RequestId.create(Date.now()),
            calls: [connection.call("revoke", { scope: fixture.scope.id, id: reference.id })],
        })
        .then(([written]) => written);

    expect({
        revoked: fixture.server.revoked,
        authorized: await authorized(fixture, reference),
        credential: await fixture.tokens.read(fixture.objects, {
            id: reference.id,
            vault: fixture.vaultReference,
        }),
        lent: await refusal(lend(fixture, reference)),
    }).toEqual({
        revoked: ["refresh-1"],
        authorized: ["false", "Revoked", false],
        credential: {},
        lent: ["UNAUTHORIZED", `connection ${reference.id} is not authorized`],
    });
});

test("send a key connection's key in its header, keeping the key out of the rows and the journal", async () => {
    const fixture = await open();
    const key = "sk-test-0123456789";
    const reference = await create(fixture, {
        protocol: { kind: "key", header: "x-api-key" },
        material: { kind: "key", key },
    });
    await fixture.control();
    const loan = await fixture.lender.lend(
        reference,
        new Request("https://api.test/items"),
        Date.now(),
    );

    // list the tables whose rows hold the key
    const leaking: string[] = [];
    for (const [name, rows] of Object.entries(await fixture.dump())) {
        if (JSON.stringify(rows).includes(key)) {
            leaking.push(name);
        }
    }

    expect({
        header: loan.request.headers.get("x-api-key"),
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
        const credential = await fixture.tokens.read(fixture.objects, {
            id: reference.id,
            vault: fixture.vaultReference,
        });
        const token = present(credential.token, "the token");
        await fixture.tokens.write(
            fixture.objects,
            { id: reference.id, vault: fixture.vaultReference },
            { ...credential, token: { ...token, refreshToken, expiresAt: Date.now() } },
        );
        await fixture.database.update(connection.table).set({ expiresAt: Date.now() });
    };
    await expire("refresh-1");
    await fixture.control();
    const refreshed = [await lend(fixture, reference), await authorized(fixture, reference)];

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
        .mutate(fixture.context().caller(), {
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

    // read every object through the service's reading procedures
    const read = { scope: fixture.scope.id };
    const answers = JSON.stringify([
        await fixture.objects.query(fixture.context().caller(), connection.call("list", read)),
        await fixture.objects.query(
            fixture.context().caller(),
            connection.call("get", { ...read, id: reference.id }),
        ),
        await fixture.objects.query(fixture.context().caller(), integration.call("list", read)),
        await fixture.objects.query(
            fixture.context().caller(),
            authorizationRequest.call("list", read),
        ),
    ]);

    // list the tokens, client secret, pending verifier and session an answer holds
    const pending = aligned(await fixture.database.select().from(authorizationRequest.table), 0);
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
        .mutate(fixture.context().caller(), {
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
        .mutate(fixture.context().caller(), {
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
