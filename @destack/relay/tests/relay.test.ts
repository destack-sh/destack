import { afterAll, beforeAll, expect, test } from "@destack/test";
import { AccountFixture, ids, TestDirectory } from "@destack/account/test";
import { machine } from "@destack/account/object";
import { and, eq } from "@destack/db";
import { Identifier, present } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import type { Subject } from "@destack/sync";
import { RELAY_TOKEN_LIFETIME_MILLISECONDS } from "../src/server/index.ts";
import { TUNNEL_PROTOCOL, TunnelProtocol } from "../src/session/index.ts";
import { RelayFixture } from "../src/test/index.ts";
import { Simulation } from "@destack/environment/simulation";

/** Describe the report of a socket close a relay refused while its replicas were stale. */
function refusedClose(id: string) {
    return {
        scope: "@destack/runtime",
        name: "Error",
        message: `instance instance-relay refused the close of socket ${id}: 503`,
    };
}

/** The simulation every scenario runs in. */
const simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 16) });

/** The account package with each scenario's enrolled machine. */
let accounts: AccountFixture;

/** The relay installation in the residency's home space. */
let installation: Subject;

beforeAll(async () => {
    simulation.install();
    // write and serve the account package inside the simulation, which runs only in-process effects
    accounts = await AccountFixture.create({
        dialect: "sqlite",
        signIn: { clock: simulation.clock },
    });
    installation = await accounts.install();
});

afterAll(async () => {
    try {
        await accounts[Symbol.asyncDispose]();
    } finally {
        simulation.uninstall();
    }
});

test("forward a request for a space's name through its machine's tunnel, preserving its name, method and body", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);

    // call the machine's app by the space's name
    expect(
        await fixture.request(relay, fixture.notes, "/notes/1", { method: "POST", body: "pinned" }),
    ).toBe("hello POST /notes/1 pinned");
    expect([fixture.received, fixture.telemetry.exceptions()]).toEqual([
        [`POST ${fixture.notes}/notes/1`],
        [],
    ]);
});

test("route a machine's name within its account through its tunnel", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);

    // call the machine itself, and nothing by a name no standing machine of the account has
    expect([
        await fixture.request(relay, await fixture.computer(), "/status"),
        await fixture.request(relay, "missing--acme.destack.computer", "/status"),
    ]).toEqual(["hello GET /status", "404 NOT_FOUND"]);
});

test("tell a machine its name as its tunnel opens, and again at once after its rename", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay, { heartbeat: 60_000 });
    const before = await fixture.computer();
    await fixture.until(async () => fixture.names.length > 0);

    // rename the machine
    await accounts.machine(fixture.identity).machine.rename({
        scope: ids.account,
        id: fixture.machineId,
        requestId: RequestId.create(Date.now()),
        name: "renamed",
    });

    // call the machine by its new name once the relay tells it
    await fixture.until(async () => fixture.names.at(-1) === "renamed--acme.destack.computer");
    expect([
        fixture.names,
        await fixture.request(relay, "renamed--acme.destack.computer", "/status"),
        await fixture.request(relay, before, "/status"),
        fixture.telemetry.exceptions(),
    ]).toEqual([
        [before, "renamed--acme.destack.computer"],
        "hello GET /status",
        "404 NOT_FOUND",
        [],
    ]);
});

test("refuse names that lead nowhere, and a named machine that has no tunnel", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();

    // refuse other handles, spaces, shapes and domains, and the machine before it connects
    expect(
        await Promise.all(
            [
                `notes--${fixture.space}--rival.destack.space`,
                "notes--missing--acme.destack.space",
                `${fixture.space}--acme.destack.space`,
                `notes.${fixture.space}.acme.example.com`,
                fixture.notes,
            ].map((name) => fixture.request(relay, name, "/")),
        ),
    ).toEqual([
        "404 NOT_FOUND",
        "404 NOT_FOUND",
        "404 NOT_FOUND",
        "404 NOT_FOUND",
        "503 SERVICE_UNAVAILABLE",
    ]);

    // answer the refused request to the machine without a tunnel as a refusal, capturing nothing
    expect(fixture.telemetry.take()).toEqual([]);
});

test("refuse a tunnel without a valid machine token among its subprotocols", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();

    // offer a forged token, a valid one only in the authorization header, and one without the tunnel protocol
    const upgrade = (headers: Record<string, string>) =>
        fixture.network
            .of("machine")
            .fetch(
                new Request(relay.url, {
                    headers: { upgrade: "websocket", connection: "upgrade", ...headers },
                }),
            )
            .then((response) => response.status);
    const token = await fixture.token();
    const [, bearer = ""] = TunnelProtocol.offer(token);
    expect([
        await upgrade({ "sec-websocket-protocol": TunnelProtocol.offer("forged").join(", ") }),
        await upgrade({ authorization: `Bearer ${token}` }),
        await upgrade({ "sec-websocket-protocol": bearer }),
    ]).toEqual([401, 401, 401]);
});

test("open a tunnel with the token offered as a subprotocol, answering the tunnel protocol only", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();

    // open a WebSocket as the machine, and read the protocol the relay chose
    const socket = fixture.network
        .of("machine")
        .connect(relay.url.replace("http:", "ws:"), TunnelProtocol.offer(await fixture.token()));
    const protocol = await new Promise<string>((resolve, reject) => {
        socket.addEventListener("open", () => resolve(socket.protocol));
        socket.addEventListener("close", () => reject(new Error("the relay refused the tunnel")));
    });
    socket.close();
    expect(protocol).toBe(TUNNEL_PROTOCOL);
});

test("restore the route when the machine reconnects after its relay restarts", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const first = await fixture.relay();
    await fixture.tunnel(first);

    // restart the relay on the same port, and wait for the machine to dial it again
    await fixture.stop(first);
    const second = await fixture.relay({ port: first.port });
    await fixture.until(
        async () => (await fixture.request(second, fixture.notes, "/again")) === "hello GET /again",
    );
});

test("route a branch to its space's machine under the branch's name", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);

    const branch = `notes--feature-x---${fixture.space}--acme.destack.space`;
    expect([await fixture.request(relay, branch, "/"), fixture.received]).toEqual([
        "hello GET /",
        [`GET ${branch}/`],
    ]);
});

test("forward a name a cloud machine serves to the machine's endpoint, preserving the host asked for", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    const cloud = `cloud-${fixture.space.slice(-12)}`;
    await fixture.cloud("cloud");
    await RelayFixture.place(fixture.accounts, cloud, ids.machine);
    await fixture.settle(relay);

    const name = `notes--${cloud}--acme.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`cloud host="${name}";proto=http`);
});

test("route the name of a space whose account lives in another residency through the directory", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();

    // create an account in the us residency, with a space on the us cloud machine
    const abroad = await accounts.user(ids.owner).account.create({
        scope: ids.owner,
        requestId: RequestId.create(Date.now()),
        handle: `abroad-${fixture.space.slice(-8)}`,
        name: "Abroad",
        kind: "shared",
        residencyId: "us",
    });
    const [cloud] = await accounts.database
        .select({ id: machine.table.id })
        .from(machine.table)
        .where(and(eq(machine.table.residencyId, "us"), eq(machine.table.kind, "cloud")));
    const machineId = present(cloud, "the us cloud machine").id;
    await fixture.cloud("us", machineId);
    await RelayFixture.place(accounts, fixture.space, machineId, abroad.id);
    await fixture.settle(relay);

    // route the space's name from the eu relay to the us cloud machine
    const name = `notes--${fixture.space}--${abroad.handle}.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`us host="${name}";proto=http`);
});

test("route a request from an eu relay to a space on a personal machine tunnelled to a us relay", async () => {
    // create an account in the us residency, with a personal machine serving a space
    const abroad = await accounts.user(ids.owner).account.create({
        scope: ids.owner,
        requestId: RequestId.create(Date.now()),
        handle: `tunnelled-${Date.now().toString(36)}`,
        name: "Tunnelled",
        kind: "shared",
        residencyId: "us",
    });
    await using fixture = await RelayFixture.open(simulation, accounts, installation, abroad.id);

    // tunnel the machine to the us relay, and start the eu relay once the endpoint is published
    const us = await fixture.relay({
        installation: await accounts.install([], "us"),
    });
    await fixture.tunnel(us);
    await fixture.until(async () => (await fixture.endpoint()) !== null);
    const eu = await fixture.relay();

    // forward the space's name from the eu relay through the us relay to the machine
    const name = `notes--${fixture.space}--${abroad.handle}.destack.space`;
    expect([await fixture.request(eu, name, "/eu?x=1"), fixture.received]).toEqual([
        "hello GET /eu",
        [`GET ${name}/eu`],
    ]);
});

test("route a request from an eu relay to a machine's name in the us residency through the us relay with its tunnel, until the tunnel lapses", async () => {
    // create an account in the us residency, with a personal machine
    const abroad = await accounts.user(ids.owner).account.create({
        scope: ids.owner,
        requestId: RequestId.create(Date.now()),
        handle: `computer-${Date.now().toString(36)}`,
        name: "Computer",
        kind: "shared",
        residencyId: "us",
    });
    await using fixture = await RelayFixture.open(simulation, accounts, installation, abroad.id);
    const { name } = await accounts
        .machine(fixture.identity)
        .machine.get({ scope: abroad.id, id: fixture.machineId });

    // tunnel the machine to the us relay, and start the eu relay once the endpoint is published
    const us = await fixture.relay({
        installation: await accounts.install([], "us"),
    });
    await fixture.tunnel(us);
    await fixture.until(async () => (await fixture.endpoint()) !== null);
    const eu = await fixture.relay();

    // forward the machine's name from the eu relay, which replicates every residency's machines, through the us relay
    const computer = `${name}--${abroad.handle}.destack.computer`;
    const forwarded = await fixture.request(eu, computer, "/status");

    // refuse the name at the eu relay once the published tunnel lapsed
    const opened = await fixture.connections(us);
    await fixture.advance(RELAY_TOKEN_LIFETIME_MILLISECONDS);
    const lapsed = await fixture.request(eu, computer, "/status");
    expect([forwarded, lapsed, fixture.received]).toEqual([
        "hello GET /status",
        "503 SERVICE_UNAVAILABLE",
        [`GET ${computer}/status`],
    ]);

    // report the close the stale relay refused, and let the lapse remove its connection
    const isRemoved = async () =>
        (await fixture.connections(us)).every((id) => !opened.includes(id));
    await fixture.until(
        async () => (await isRemoved()) && fixture.telemetry.exceptions().length === opened.length,
    );
    expect(fixture.telemetry.take()).toEqual(opened.map(refusedClose));
});

test("leave a machine's endpoint to the machine, writing none as the relay opens, routes and closes its tunnel", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay, { renewed: (renewal) => fixture.names.push(renewal.name) });
    const computer = await fixture.computer();
    const routed = await fixture.request(relay, computer, "/");
    await fixture.until(async () => fixture.names.length > 0);

    // expect no endpoint of the machine at the account package once the relay closes
    await fixture.stop(relay);
    expect([
        routed,
        fixture.names,
        await fixture.endpoint(),
        fixture.telemetry.exceptions(),
    ]).toEqual(["hello GET /", [computer], null, []]);
});

test("follow a space's move from its machine to a cloud machine", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // move the space to the cloud machine at the next epoch
    await fixture.cloud("cloud");
    const placement = {
        id: fixture.spaceId,
        scope: ids.account,
        machine: fixture.machineId,
        epoch: 1,
    };
    await TestDirectory.place(fixture.accounts.database, placement, {
        target: ids.machine,
        transferId: Identifier.create("transfer"),
    });
    await TestDirectory.place(fixture.accounts.database, {
        ...placement,
        machine: ids.machine,
        epoch: 2,
    });

    await fixture.until(async () =>
        (await fixture.request(relay, fixture.notes, "/")).startsWith("cloud "),
    );
});

test("follow a space's rename", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // place the space under a new name, releasing the old one
    const renamed = `renamed-${fixture.space.slice(-12)}`;
    await fixture.rename(renamed);

    // refuse the old name and call the space by the new one
    await fixture.until(
        async () => (await fixture.request(relay, fixture.notes, "/")) !== "hello GET /",
    );
    expect([
        await fixture.request(relay, fixture.notes, "/"),
        await fixture.request(relay, `notes--${renamed}--acme.destack.space`, "/"),
    ]).toEqual(["404 NOT_FOUND", "hello GET /"]);
});

test("resolve a handle from the relay's replica after the account package renames it", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // rename the account's handle as its owner, and rename it back afterwards for the other scenarios
    const owner = accounts.user(ids.owner);
    const rename = async (handle: string) => {
        const current = await owner.account.get({ scope: ids.owner, id: ids.account });
        await owner.account.update({
            scope: ids.owner,
            requestId: RequestId.create(Date.now()),
            id: ids.account,
            revision: current.revision,
            handle,
        });
    };
    await rename("acme-renamed");
    try {
        // refuse the old handle and call the space by the new one
        await fixture.until(
            async () => (await fixture.request(relay, fixture.notes, "/")) !== "hello GET /",
        );
        expect([
            await fixture.request(relay, fixture.notes, "/"),
            await fixture.request(relay, fixture.notes.replace("--acme.", "--acme-renamed."), "/"),
        ]).toEqual(["404 NOT_FOUND", "hello GET /"]);
    } finally {
        await rename("acme");
    }
});

test("follow a machine's revocation, closing its tunnel at once, refusing it again and recording the opening and the close", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);
    const computer = await fixture.computer();
    expect(await fixture.request(relay, computer, "/")).toBe("hello GET /");

    // revoke the machine as its owner
    await accounts.user(ids.owner).machine.revoke({
        scope: ids.account,
        id: fixture.machineId,
        requestId: RequestId.create(Date.now()),
    });

    // refuse its name and its space's name once its tunnel closes early
    await fixture.until(
        async () => (await fixture.request(relay, computer, "/")) === "404 NOT_FOUND",
    );
    await fixture.until(
        async () =>
            (await fixture.request(relay, fixture.notes, "/")) === "503 SERVICE_UNAVAILABLE",
    );

    // journal the opening under the machine's name and the close with its reason, as the relay in its home
    await fixture.until(async () =>
        (await fixture.journaled()).some((call) => call.method === "tunnel.close"),
    );
    expect(
        (await fixture.journaled())
            .filter((call) => call.method.startsWith("tunnel."))
            .map((call) => [
                call.method,
                call.execution.context.scope,
                call.execution.context.caller.subject.type,
                call.input["name"] ?? call.input["reason"],
            ]),
    ).toEqual([
        ["tunnel.open", installation.scope, "installation", computer],
        ["tunnel.close", installation.scope, "installation", "machine-revoked"],
    ]);
});

test("close a machine's tunnel once its last key is revoked, recording why", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);
    const computer = await fixture.computer();
    expect(await fixture.request(relay, computer, "/")).toBe("hello GET /");

    // revoke the machine's only key as its owner
    const owner = accounts.user(ids.owner);
    const { items } = await owner.key.list({
        scope: ids.account,
        where: { parentId: fixture.machineId },
    });
    for (const key of items) {
        await owner.key.revoke({
            scope: ids.account,
            id: key.id,
            requestId: RequestId.create(Date.now()),
        });
    }

    // close the tunnel at once, recording the revoked key as the reason
    await fixture.until(
        async () =>
            (await fixture.request(relay, fixture.notes, "/")) === "503 SERVICE_UNAVAILABLE",
    );
    await fixture.until(async () =>
        (await fixture.journaled()).some((call) => call.method === "tunnel.close"),
    );
    expect(
        (await fixture.journaled())
            .filter((call) => call.method === "tunnel.close")
            .map((call) => call.input["reason"]),
    ).toEqual(["key-revoked"]);
});

test("close a disabled machine's tunnel, and route to it again once it is enabled and reconnects", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    await fixture.tunnel(relay);
    const computer = await fixture.computer();
    expect(await fixture.request(relay, computer, "/")).toBe("hello GET /");

    // disable the machine as its owner, suspending its keys and closing its tunnel
    const owner = accounts.user(ids.owner);
    const target = { scope: ids.account, id: fixture.machineId };
    await owner.machine.disable({ ...target, requestId: RequestId.create(Date.now()) });
    await fixture.until(async () =>
        (await fixture.journaled()).some((call) => call.method === "tunnel.close"),
    );

    // enable it again, and route to it once it reconnects through its tunnel
    await owner.machine.enable({ ...target, requestId: RequestId.create(Date.now()) });
    await fixture.until(
        async () => (await fixture.request(relay, computer, "/")) === "hello GET /",
    );
    expect(
        (await fixture.journaled())
            .filter((call) => call.method.startsWith("tunnel."))
            .map((call) => call.method),
    ).toEqual(["tunnel.open", "tunnel.close"]);
});

test("end a machine's connection once its token lapses, and route to the machine again once it reconnects", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();

    // offer the first token alone until told otherwise, so the machine cannot renew before the lapse
    const first = await fixture.token();
    let isRenewable = false;
    await fixture.tunnel(relay, {
        token: () => (isRenewable ? fixture.token() : Promise.resolve(first)),
    });
    const computer = await fixture.computer();
    const connected = await fixture.request(relay, computer, "/");

    // wait for the token's lapse, then lapse the connection past it, reporting its refused close
    const opened = await fixture.connections(relay);
    const wait = await (await relay.tunnel(fixture.machineId)).lapse();
    await fixture.advance(RELAY_TOKEN_LIFETIME_MILLISECONDS);
    const lapsed = await (await relay.tunnel(fixture.machineId)).lapse();
    await fixture.until(() =>
        Promise.resolve(fixture.telemetry.exceptions().length === opened.length),
    );
    const refused = fixture.telemetry.take();

    // route again once the machine reconnects with a new token
    isRenewable = true;
    await fixture.until(
        async () => (await fixture.request(relay, computer, "/")) === "hello GET /",
    );
    expect([connected, wait !== undefined && wait > 0, lapsed, refused]).toEqual([
        "hello GET /",
        true,
        undefined,
        opened.map(refusedClose),
    ]);
});

test("follow a cloud machine's new endpoint", async () => {
    await using fixture = await RelayFixture.open(simulation, accounts, installation);
    const relay = await fixture.relay();
    const cloud = `cloud-${fixture.space.slice(-12)}`;
    await fixture.cloud("first");
    await RelayFixture.place(fixture.accounts, cloud, ids.machine);
    await fixture.settle(relay);
    const name = `notes--${cloud}--acme.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`first host="${name}";proto=http`);

    // publish the cloud machine's new endpoint
    await fixture.cloud("second");

    await fixture.until(async () =>
        (await fixture.request(relay, name, "/")).startsWith("second "),
    );
});
