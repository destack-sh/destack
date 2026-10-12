import { principal } from "@destack/access";
import { inArray } from "@destack/db";
import { aligned, present, Clock } from "@destack/schema";
import { ControlLoop } from "@destack/service/control";
import { RequestId } from "@destack/service/request";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Replica, type ObjectReference } from "@destack/sync";
import { journal } from "@destack/audit/stack";
import { Credential } from "../credential/credential.ts";
import { authorizationRequest, connection, integration } from "../object/index.ts";
import { ConnectionFixture, RESOURCE } from "../test/index.ts";

/** The redirect URI the test integration registered. */
const REDIRECT = "https://app.test/callback";

/** Open the service with an integration and a space copying it, both until the test ends. */
async function open(spaceId = "space-two") {
    // keep the integration at its home
    const home = await ConnectionFixture.open();
    onTestFinished(() => home.close());
    const registered = await home.staticIntegration();

    // copy it into a space, following the home's integrations in a control loop
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const space = await follow(home, spaceId, controller.signal);

    return { home, space, registered, signal: controller.signal };
}

/** Open a space copying a home's integrations in a control loop until the test ends. */
async function follow(
    home: ConnectionFixture,
    spaceId: string,
    signal: AbortSignal,
    enclosing?: ObjectReference,
): Promise<ConnectionFixture> {
    // place the space in the home's scope, and serve its connections over the home's integrations
    await home.place(spaceId);
    const space = await ConnectionFixture.open({
        home: {
            home: home.home(spaceId),
            space: spaceId,
            ...(enclosing === undefined ? {} : { enclosing }),
        },
    });
    onTestFinished(() => space.close());

    // follow the copies until the test ends, keeping every failure
    const controller = new AbortController();
    signal.addEventListener("abort", () => controller.abort());
    const failures: unknown[] = [];
    const loop = new ControlLoop(
        space.database,
        space.objects.controllers().filter((each) => each.name === "replica"),
        { clock: Clock.system, report: (_controller, _key, error) => failures.push(error) },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
        expect(failures).toEqual([]);
    });

    return space;
}

/** Create a connection through the copied integration in a space, with its external account. */
async function connect(
    fixture: Awaited<ReturnType<typeof open>>,
    space: ConnectionFixture,
    input: {
        readonly subject?: string;
        readonly kind?: "authorization-code" | "client-credentials";
    } = {},
): Promise<ObjectReference> {
    // create the connection referencing the home's integration
    const created = await space.objects
        .mutate(space.context().caller(), {
            id: RequestId.create(Date.now()),
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

    // wait for the copy of the integration to hold the home's head
    const head = await fixture.home.database.log.position();
    await Replica.wait(space.database, fixture.registered.scope, head, fixture.signal);

    return connection.reference(space.scope.id, created.id);
}

/** Start authorizing a space's connection as its owner, redirected to the test redirect URI. */
function authorize(space: ConnectionFixture, id: string) {
    return space.objects
        .mutate(space.context().caller(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("authorize", { scope: space.scope.id, id, redirectUri: REDIRECT }),
            ],
        })
        .then(([written]) => written);
}

/** Complete a connection's authorization in a space as its owner with a redirect's parameters. */
function complete(space: ConnectionFixture, id: string, parameters: Record<string, string>) {
    return space.objects
        .mutate(space.context().caller(), {
            id: RequestId.create(Date.now()),
            calls: [connection.call("complete", { scope: space.scope.id, id, parameters })],
        })
        .then(([written]) => written);
}

/** Create a client credentials connection at home, acquire its token, and bind it to a consumer. */
async function bindAtHome(
    home: ConnectionFixture,
    registered: ObjectReference,
    consumer: ObjectReference,
): Promise<ObjectReference> {
    // create the connection and acquire its token
    const created = await home.objects
        .mutate(home.context().caller(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("create", {
                    scope: home.scope.id,
                    vault: home.vaultReference,
                    protocol: { kind: "client-credentials" },
                    integration: registered,
                }),
            ],
        })
        .then(([written]) => written);
    await home.control();

    // bind it to the consumer
    await home.objects
        .mutate(home.context().caller(), {
            id: RequestId.create(Date.now()),
            calls: [
                connection.call("bindConsumer", { scope: home.scope.id, id: created.id, consumer }),
            ],
        })
        .then(([written]) => written);

    return connection.reference(home.scope.id, created.id);
}

test("authorize a connection through an integration another service keeps, which authenticates the client and keeps none of the tokens", async () => {
    const fixture = await open();
    const { home, space } = fixture;
    const reference = await connect(fixture, space);

    // authorize at the home's server and complete in the space, the code exchanged through the home
    const { url } = await authorize(space, reference.id);
    await complete(space, reference.id, await home.server.approve(url));
    const loan = await space.lender.lend(
        reference,
        new Request("https://api.test/items"),
        Date.now(),
    );

    // read the space's connection, the home's connections and pending authorizations, and the home's journal of token requests
    const [row] = await space.connections();
    const entries = await home.database.select().from(journal);
    const requested = entries.filter((entry) => entry.method === "integration.requestToken");

    // keep the tokens in the space's vault alone, the client secret at home alone, and no token in the home's rows or journal
    expect({
        lent: loan.request.headers.get("authorization"),
        token: home.server.requests.map((request) => [request.path, request.body["grant_type"]]),
        spaceVault: (await space.secrets()).map((value) => Credential.read(value)),
        homeVault: await home.secrets(),
        homeConnections: await home.connections(),
        homeRequests: await home.database.select().from(authorizationRequest.table),
        homeJournal: requested.map((entry) => [entry.call.input, entry.call.execution.outcome]),
    }).toEqual({
        lent: "Bearer access-1",
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
        homeVault: [home.server.clientSecret],
        homeConnections: [],
        homeRequests: [],
        homeJournal: [[{}, { kind: "success" }]],
    });
});

test("refuse client credentials through a copied integration, and token requests from anyone but a space", async () => {
    const fixture = await open();
    const { home, space, registered } = fixture;
    const reference = await connect(fixture, space, { kind: "client-credentials" });
    await space.control();
    const [row] = await space.database.select().from(connection.table);

    // ask the home as a person, outside the spaces
    const asPerson = await refusal(
        home.objects.query(
            home.context().caller(),
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
        requested: home.server.requests,
    }).toEqual({
        connection: [
            reference.id,
            `a client_credentials grant proves no possession, so only the home of integration ${registered.id} requests it`,
        ],
        asPerson: ["FORBIDDEN", "permission denied: request"],
        requested: [],
    });
});

test("register the client of a space's connection without an integration at the home of its account, and authorize through the copy", async () => {
    const fixture = await open();
    const { home, signal } = fixture;
    const space = await follow(home, "space-three", signal, home.scope);

    // create a connection with only its protected resource in a space of the home's scope
    const created = await space.objects
        .mutate(space.context().caller(), {
            id: RequestId.create(Date.now()),
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

    // register at home, wait for the copy, and complete through the home's client
    const { url } = await authorize(space, created.id);
    const head = await home.database.log.position();
    await Replica.wait(space.database, home.scope.id, head, signal);
    await complete(space, created.id, await home.server.approve(url));
    const [row] = await space.connections();
    const kept = await home.database.select().from(integration.table);
    const reference = connection.reference(space.scope.id, created.id);
    const loan = await space.lender.lend(
        reference,
        new Request("https://api.test/items"),
        Date.now(),
    );

    expect({
        integration: row?.integration,
        kept: kept.map((each) => [each.scope, each.registration]),
        spaceServer: space.server.requests,
        lent: loan.request.headers.get("authorization"),
    }).toEqual({
        integration: integration.reference(home.scope.id, aligned(kept, 1).id),
        kept: [
            [home.scope.id, "static"],
            [home.scope.id, "dynamic"],
        ],
        spaceServer: [],
        lent: "Bearer access-1",
    });
});

test("lend a connection its home keeps to a space's bound installation through the token the home issues, refusing an unbound one, and keep installation connections at the home", async () => {
    const fixture = await open();
    const { home, space, registered } = fixture;

    // acquire a client credentials token at home and bind the connection to an installation of the space
    const bound = principal.installation.reference(space.scope.id, "installation-bridge");
    const other = principal.installation.reference(space.scope.id, "installation-other");
    const reference = await bindAtHome(home, registered, bound);

    // lend it from the space to each installation, and create an installation connection in the space
    const lend = (consumer: typeof bound) =>
        refusal(
            space.lender.lend(
                reference,
                new Request("https://api.test/items"),
                Date.now(),
                consumer,
            ),
        );
    const installed = await refusal(
        space.objects
            .mutate(space.context().caller(), {
                id: RequestId.create(Date.now()),
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

    const loan = await space.lender.lend(
        reference,
        new Request("https://api.test/items"),
        Date.now(),
        bound,
    );

    expect({
        lent: loan.request.headers.get("authorization"),
        unbound: await lend(other),
        installed,
    }).toEqual({
        lent: "Bearer access-1",
        // hide the home's scope from an installation the connection is not bound to
        unbound: ["NOT_FOUND", `no scope ${home.scope.id}`],
        installed: ["BAD_REQUEST", "installation connections live at their integration's home"],
    });
});
