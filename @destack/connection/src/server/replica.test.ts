import { principal } from "@destack/access";
import { inArray } from "@destack/db";
import { present, Identifier } from "@destack/schema";
import { Replica, RequestId } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import type { ObjectReference } from "@destack/sync";
import { journal } from "@destack/object";
import { Credential } from "../credential/credential.ts";
import { authorizationRequest, connection, integration } from "../object/index.ts";
import { TestConnection, RESOURCE } from "../test/index.ts";

/** The redirect URI the test integration registered. */
const REDIRECT = "https://app.test/callback";

/** Serve connections with an integration and a space replicating it, both until the test ends. */
async function open(given?: string) {
    // store the integration at its primary, and name the space in the primary's world
    const primary = await TestConnection.open();
    onTestFinished(() => primary.close());
    const registered = await primary.staticIntegration();
    const spaceId = given ?? Identifier.create("space", primary.environment);

    // replicate it into a space, following the primary's integrations in a control loop
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const space = await follow(primary, spaceId, controller.signal);

    return { primary, space, registered, signal: controller.signal };
}

/** Open a space in a primary's world, replicating its account in a loop until the test ends. */
async function follow(
    primary: TestConnection,
    spaceId: string,
    signal: AbortSignal,
): Promise<TestConnection> {
    // place the space in the primary's account, and serve its connections over the primary's integrations
    const placed = await primary.place(spaceId);
    const space = await TestConnection.open({
        primary: { ...primary.primary(spaceId), space: spaceId, enclosing: placed },
        simulation: primary.simulation,
    });
    onTestFinished(() => space.close());

    // follow the replicas until the test ends, moving the world until they reach the primary's head
    space.follow();
    const head = await primary.database.log.position();
    await Replica.wait(space.database, primary.scope.id, head, signal);

    return space;
}

/** Create a connection through the replicated integration in a space, with its external account. */
async function connect(
    fixture: Awaited<ReturnType<typeof open>>,
    space: TestConnection,
    input: {
        readonly subject?: string;
        readonly kind?: "authorization-code" | "client-credentials";
    } = {},
): Promise<ObjectReference> {
    // create the connection referencing the primary's integration
    const created = await space.objects
        .mutate(space.context().access(), {
            id: RequestId.create(space.environment),
            calls: [
                connection.call("create", {
                    scope: space.scope.id,
                    vault: space.vaultReference,
                    protocol: { kind: input.kind ?? "authorization-code" },
                    integration: fixture.registered,
                }),
            ],
        })
        .then(([written]) => written);
    if (input.subject !== undefined) {
        await space.database
            .update(connection.table)
            .set({ subject: input.subject })
            .where(inArray(connection.table.id, [created.id]));
    }

    // wait for the replica of the integration to match the primary's head
    const head = await fixture.primary.database.log.position();
    await space.objects.wait({ scope: fixture.registered.scope, ...head }, fixture.signal);

    return connection.reference(space.scope.id, created.id);
}

/** Start authorizing a space's connection as its owner, redirected to the test redirect URI. */
function authorize(space: TestConnection, id: string) {
    return space.objects
        .mutate(space.context().access(), {
            id: RequestId.create(space.environment),
            calls: [
                connection.call("authorize", { scope: space.scope.id, id, redirectUri: REDIRECT }),
            ],
        })
        .then(([written]) => written);
}

/** Complete a connection's authorization in a space as its owner with a redirect's parameters. */
function complete(space: TestConnection, id: string, parameters: Record<string, string>) {
    return space.objects
        .mutate(space.context().access(), {
            id: RequestId.create(space.environment),
            calls: [connection.call("complete", { scope: space.scope.id, id, parameters })],
        })
        .then(([written]) => written);
}

/** Create a client credentials connection at the primary, and bind its token to a consumer. */
async function bindAtPrimary(
    primary: TestConnection,
    registered: ObjectReference,
    consumer: ObjectReference,
): Promise<ObjectReference> {
    // create the connection and acquire its token
    const created = await primary.objects
        .mutate(primary.context().access(), {
            id: RequestId.create(primary.environment),
            calls: [
                connection.call("create", {
                    scope: primary.scope.id,
                    vault: primary.vaultReference,
                    protocol: { kind: "client-credentials" },
                    integration: registered,
                }),
            ],
        })
        .then(([written]) => written);
    await primary.control();

    // bind it to the consumer
    await primary.objects
        .mutate(primary.context().access(), {
            id: RequestId.create(primary.environment),
            calls: [
                connection.call("bindConsumer", {
                    scope: primary.scope.id,
                    id: created.id,
                    consumer,
                }),
            ],
        })
        .then(([written]) => written);

    return connection.reference(primary.scope.id, created.id);
}

test("authorize a connection through an integration another space stores, which authenticates the client and stores none of the tokens", async () => {
    const fixture = await open();
    const { primary, space } = fixture;
    const reference = await connect(fixture, space);

    // authorize at the primary's server and complete in the space, the code exchanged through the primary
    const { url } = await authorize(space, reference.id);
    await complete(space, reference.id, await primary.server.approve(url));
    const leased = await space.bindAndLease(reference, new Request("https://api.test/items"));

    // read the space's connection, the primary's connections and pending authorizations, and the primary's journal of token requests
    const [row] = await space.connections();
    const entries = await primary.database.select().from(journal);
    const requested = entries.filter((entry) => entry.method === "integration.requestToken");

    // store the tokens in the space's vault alone, the client secret at the primary alone, and no token in the primary's rows or journal
    expect({
        leased: leased.request.headers.get("authorization"),
        token: primary.server.requests.map((request) => [request.path, request.body["grant_type"]]),
        spaceVault: (await space.secrets()).map((value) => Credential.read(value)),
        primaryVault: await primary.secrets(),
        primaryConnections: await primary.connections(),
        primaryRequests: await primary.database.select().from(authorizationRequest.table),
        primaryJournal: requested.map((entry) => [entry.call.input, entry.call.execution.outcome]),
    }).toEqual({
        leased: "Bearer access-1",
        token: [["https://auth.test/token", "authorization_code"]],
        spaceVault: [
            {
                token: {
                    accessToken: "access-1",
                    tokenType: "bearer",
                    refreshToken: "refresh-1",
                    expiresAt: present(row, "the space's connection").expiresAt,
                },
            },
        ],
        primaryVault: [primary.server.clientSecret],
        primaryConnections: [],
        primaryRequests: [],
        primaryJournal: [[{}, { kind: "success" }]],
    });
});

test("refuse client credentials through a replicated integration, and token requests from anyone but a space", async () => {
    const fixture = await open();
    const { primary, space, registered } = fixture;
    const reference = await connect(fixture, space, { kind: "client-credentials" });
    await space.control();
    const [row] = await space.database.select().from(connection.table);

    // ask the primary as a person, outside the spaces
    const asPerson = await refusal(
        primary.objects.query(
            primary.context().access(),
            integration.call("requestToken", {
                scope: registered.scope,
                id: registered.id,
                grant: { grant_type: "refresh_token", refresh_token: "refresh-1" },
            }),
        ),
    );

    expect({
        connection: [
            reference.id,
            present(row, "the connection").conditions["Authorized"]?.message,
        ],
        asPerson,
        requested: primary.server.requests,
    }).toEqual({
        connection: [
            reference.id,
            `only the primary of integration ${registered.id} requests a client_credentials grant`,
        ],
        asPerson: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: request"],
        requested: [],
    });
});

test("register the client of a space's connection without an integration at the primary of its account, and authorize through the replica", async () => {
    const fixture = await open();
    const { primary, signal } = fixture;
    const space = await follow(primary, Identifier.create("space", primary.environment), signal);

    // create a connection with only its protected resource in a space of the primary's scope
    const created = await space.objects
        .mutate(space.context().access(), {
            id: RequestId.create(space.environment),
            calls: [
                connection.call("create", {
                    scope: space.scope.id,
                    vault: space.vaultReference,
                    protocol: { kind: "authorization-code" },
                    resource: RESOURCE,
                }),
            ],
        })
        .then(([written]) => written);

    // register at the primary, wait for the replica, and complete through the primary's client
    const { url } = await authorize(space, created.id);
    const head = await primary.database.log.position();
    await space.objects.wait({ scope: primary.scope.id, ...head }, signal);
    await complete(space, created.id, await primary.server.approve(url));
    const [row] = await space.connections();
    const stored = await primary.database
        .select()
        .from(integration.table)
        .orderBy(integration.table.registration);
    const reference = connection.reference(space.scope.id, created.id);
    const leased = await space.bindAndLease(reference, new Request("https://api.test/items"));

    expect({
        integration: row?.integration,
        stored: stored.map((each) => [each.scope, each.registration]),
        spaceServer: space.server.requests,
        leased: leased.request.headers.get("authorization"),
    }).toEqual({
        integration: integration.reference(
            primary.scope.id,
            present(
                stored.find((each) => each.registration === "dynamic"),
                "the dynamic integration",
            ).id,
        ),
        stored: [
            [primary.scope.id, "dynamic"],
            [primary.scope.id, "static"],
        ],
        spaceServer: [],
        leased: "Bearer access-1",
    });
});

test("lease a connection its primary stores to a space's bound installation through the token the primary issues, refusing an unbound one, and create installation connections at the primary", async () => {
    const fixture = await open();
    const { primary, space, registered } = fixture;

    // acquire a client credentials token at the primary and bind the connection to an installation of the space
    const bound = principal.installation.reference(space.scope.id, "installation-bridge");
    const other = principal.installation.reference(space.scope.id, "installation-other");
    const reference = await bindAtPrimary(primary, registered, bound);

    // lease it from the space to each installation, and create an installation connection in the space
    const lease = (consumer: typeof bound) =>
        refusal(primary.lease(reference, new Request("https://api.test/items"), consumer));
    const installed = await refusal(
        space.objects
            .mutate(space.context().access(), {
                id: RequestId.create(space.environment),
                calls: [
                    connection.call("create", {
                        scope: space.scope.id,
                        vault: space.vaultReference,
                        protocol: { kind: "installation" },
                        integration: registered,
                    }),
                ],
            })
            .then(([written]) => written),
    );

    const leased = await primary.lease(reference, new Request("https://api.test/items"), bound);

    expect({
        leased: leased.request.headers.get("authorization"),
        unbound: await lease(other),
        installed,
    }).toEqual({
        leased: "Bearer access-1",
        // hide the primary's scope from an installation the connection is not bound to
        unbound: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${primary.scope.id} not found`],
        installed: [
            "FORBIDDEN",
            "PRIMARY_REQUIRED",
            "installation connections belong to their integration's primary",
        ],
    });
});
