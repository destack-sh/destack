import { afterAll, beforeAll, expect, test } from "@destack/test";
import { AccountFixture, ids, TestDirectory } from "@destack/account/test";
import { machine } from "@destack/account/object";
import { and, eq } from "@destack/db";
import { present } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import type { Subject } from "@destack/sync";
import { DestinationCache, RELAY_TOKEN_LIFETIME_MILLISECONDS } from "../src/server/index.ts";
import { TUNNEL_PROTOCOL, TunnelProtocol } from "../src/session/index.ts";
import { freePort, RelayFixture, until } from "../src/test/index.ts";

/** The account service with each scenario's enrolled machine. */
let accounts: AccountFixture;

/** The relay installation in the residency's home space. */
let installation: Subject;

beforeAll(async () => {
    accounts = await AccountFixture.open();
    installation = await accounts.relay();
});

afterAll(async () => {
    await accounts[Symbol.asyncDispose]();
});

test("forward a request for a space's name through its machine's tunnel, keeping its name, method and body", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // call the machine's app by the space's name
    expect(
        await fixture.request(relay, fixture.notes, "/notes/1", { method: "POST", body: "pinned" }),
    ).toBe("hello POST /notes/1 pinned");
    expect([fixture.received, fixture.reports]).toEqual([[`POST ${fixture.notes}/notes/1`], []]);
});

test("route a machine's name within its account through its tunnel", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // call the machine itself, and nothing by a name no standing machine of the account has
    expect([
        await fixture.request(relay, await fixture.computer(), "/status"),
        await fixture.request(relay, "missing--acme.destack.computer", "/status"),
    ]).toEqual(["hello GET /status", "404 NOT_FOUND"]);
});

test("tell a machine its name as its tunnel opens, and again at once after its rename", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay, { heartbeat: 60_000 });
    const before = await fixture.computer();
    await until(async () => fixture.names.length > 0);

    // rename the machine
    await accounts.machine(fixture.identity).machine.rename({
        scope: ids.account,
        id: fixture.machineId,
        requestId: RequestId.create(Date.now()),
        name: "renamed",
    });

    // call the machine by its new name once the relay tells it
    await until(async () => fixture.names.at(-1) === "renamed--acme.destack.computer");
    expect([
        fixture.names,
        await fixture.request(relay, "renamed--acme.destack.computer", "/status"),
        await fixture.request(relay, before, "/status"),
        fixture.reports,
    ]).toEqual([
        [before, "renamed--acme.destack.computer"],
        "hello GET /status",
        "404 NOT_FOUND",
        [],
    ]);
});

test("refuse names that lead nowhere, and a named machine that keeps no tunnel", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());

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
});

test("refuse a tunnel without a valid machine token among its subprotocols", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());

    // offer a forged token, a valid one only in the authorization header, and one without the tunnel protocol
    const upgrade = (headers: Record<string, string>) =>
        fetch(relay.url, {
            headers: { upgrade: "websocket", connection: "upgrade", ...headers },
        }).then((response) => response.status);
    const token = await fixture.token();
    const [, bearer = ""] = TunnelProtocol.offer(token);
    expect([
        await upgrade({ "sec-websocket-protocol": TunnelProtocol.offer("forged").join(", ") }),
        await upgrade({ authorization: `Bearer ${token}` }),
        await upgrade({ "sec-websocket-protocol": bearer }),
    ]).toEqual([401, 401, 401]);
});

test("open a tunnel with the token offered as a subprotocol, answering the tunnel protocol only", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());

    // open a WebSocket as the machine, and read the protocol the relay chose
    const socket = new WebSocket(
        relay.url.replace("http:", "ws:"),
        TunnelProtocol.offer(await fixture.token()),
    );
    const protocol = await new Promise<string>((resolve, reject) => {
        socket.addEventListener("open", () => resolve(socket.protocol));
        socket.addEventListener("close", () => reject(new Error("the relay refused the tunnel")));
    });
    socket.close();
    expect(protocol).toBe(TUNNEL_PROTOCOL);
});

test("restore the route when the machine reconnects after its relay restarts", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const port = await freePort();
    const first = await fixture.relay(port);
    await fixture.tunnel(first);

    // restart the relay on the same port, and wait for the machine to dial it again
    await fixture.stop(first);
    const second = await fixture.relay(port);
    await until(
        async () => (await fixture.request(second, fixture.notes, "/again")) === "hello GET /again",
    );
});

test("route a branch to its space's machine under the branch's name", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    const branch = `notes--feature-x---${fixture.space}--acme.destack.space`;
    expect([await fixture.request(relay, branch, "/"), fixture.received]).toEqual([
        "hello GET /",
        [`GET ${branch}/`],
    ]);
});

test("forward a name a cloud machine serves to the machine's endpoint, keeping the host asked for", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    const cloud = `cloud-${fixture.space.slice(-12)}`;
    await fixture.cloud("cloud");
    await RelayFixture.place(fixture.accounts, cloud, ids.machine);
    await fixture.settle(relay);

    const name = `notes--${cloud}--acme.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`cloud host="${name}";proto=http`);
});

test("route the name of a space whose account lives in another residency through the directory", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());

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
    await using fixture = await RelayFixture.open(accounts, installation, abroad.id);

    // tunnel the machine to the us relay, and start the eu relay once the endpoint is published
    const us = await fixture.relay(await freePort(), {
        installation: await accounts.relay("us"),
    });
    await fixture.tunnel(us);
    const eu = await fixture.relay(await freePort());

    // forward the space's name from the eu relay through the us relay to the machine
    const name = `notes--${fixture.space}--${abroad.handle}.destack.space`;
    expect([await fixture.request(eu, name, "/eu?x=1"), fixture.received]).toEqual([
        "hello GET /eu",
        [`GET ${name}/eu`],
    ]);
});

test("route a request from an eu relay to a machine's name in the us residency, tunnelled to the us relay", async () => {
    // create an account in the us residency, with a personal machine
    const abroad = await accounts.user(ids.owner).account.create({
        scope: ids.owner,
        requestId: RequestId.create(Date.now()),
        handle: `computer-${Date.now().toString(36)}`,
        name: "Computer",
        kind: "shared",
        residencyId: "us",
    });
    await using fixture = await RelayFixture.open(accounts, installation, abroad.id);
    const { name } = await accounts
        .machine(fixture.identity)
        .machine.get({ scope: abroad.id, id: fixture.machineId });

    // tunnel the machine to the us relay, and start the eu relay once the endpoint is published
    const us = await fixture.relay(await freePort(), {
        installation: await accounts.relay("us"),
    });
    await fixture.tunnel(us);
    const eu = await fixture.relay(await freePort());

    // forward the machine's name from the eu relay, which copies every residency's machines, through the us relay
    const computer = `${name}--${abroad.handle}.destack.computer`;
    expect([await fixture.request(eu, computer, "/status"), fixture.received]).toEqual([
        "hello GET /status",
        [`GET ${computer}/status`],
    ]);
});

test("follow a space's move from its machine to a cloud machine", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
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
    await TestDirectory.place(fixture.accounts.database, placement, ids.machine);
    await TestDirectory.place(fixture.accounts.database, {
        ...placement,
        machine: ids.machine,
        epoch: 2,
    });

    await until(async () =>
        (await fixture.request(relay, fixture.notes, "/")).startsWith("cloud "),
    );
});

test("follow a space's rename", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // place the space under a new name, releasing the old one
    const renamed = `renamed-${fixture.space.slice(-12)}`;
    await fixture.rename(renamed);

    // refuse the old name and call the space by the new one
    await until(async () => (await fixture.request(relay, fixture.notes, "/")) !== "hello GET /");
    expect([
        await fixture.request(relay, fixture.notes, "/"),
        await fixture.request(relay, `notes--${renamed}--acme.destack.space`, "/"),
    ]).toEqual(["404 NOT_FOUND", "hello GET /"]);
});

test("resolve a handle from the relay's copy after the account service renames it", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
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
        await until(
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
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay, { report: () => {} });
    const computer = await fixture.computer();
    expect(await fixture.request(relay, computer, "/")).toBe("hello GET /");

    // revoke the machine as its owner
    await accounts.user(ids.owner).machine.revoke({
        scope: ids.account,
        id: fixture.machineId,
        requestId: RequestId.create(Date.now()),
    });

    // refuse its name and its space's name once its tunnel closes early
    await until(async () => (await fixture.request(relay, computer, "/")) === "404 NOT_FOUND");
    await until(
        async () =>
            (await fixture.request(relay, fixture.notes, "/")) === "503 SERVICE_UNAVAILABLE",
    );

    // record the opening as the machine and the close as the relay in the machine's account
    await until(() => fixture.audited.some((call) => call.method === "tunnel.close"));
    expect(
        fixture.audited.map((call) => [
            call.method,
            call.execution.context.scope,
            call.execution.context.caller.subject.type,
            call.execution.details,
        ]),
    ).toEqual([
        ["tunnel.open", ids.account, "machine", { name: computer }],
        ["tunnel.close", ids.account, "installation", { reason: "machine-revoked" }],
    ]);
});

test("close a machine's tunnel once its last key is revoked, recording why", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay, { report: () => {} });
    const computer = await fixture.computer();
    expect(await fixture.request(relay, computer, "/")).toBe("hello GET /");

    // revoke the machine's only key as its owner
    const owner = accounts.user(ids.owner);
    const { items } = await owner.key.list({
        scope: ids.account,
        where: { parentId: fixture.machineId },
    });
    for (const kept of items) {
        await owner.key.revoke({
            scope: ids.account,
            id: kept.id,
            requestId: RequestId.create(Date.now()),
        });
    }

    // close the tunnel at once, recording the revoked key as the reason
    await until(
        async () =>
            (await fixture.request(relay, fixture.notes, "/")) === "503 SERVICE_UNAVAILABLE",
    );
    await until(() => fixture.audited.some((call) => call.method === "tunnel.close"));
    expect(
        fixture.audited
            .filter((call) => call.method === "tunnel.close")
            .map((call) => call.execution.details),
    ).toEqual([{ reason: "key-revoked" }]);
});

test("end a machine's connection once its token lapses, and route to the machine again once it reconnects", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay, { report: () => {} });
    const computer = await fixture.computer();
    const connected = await fixture.request(relay, computer, "/");

    // wait for the token's lapse, then lapse the connection past it
    const wait = await relay.service.tunnels.lapse(fixture.machineId);
    fixture.skew = RELAY_TOKEN_LIFETIME_MILLISECONDS;
    const lapsed = await relay.service.tunnels.lapse(fixture.machineId);
    fixture.skew = 0;

    // route again once the machine reconnects with a new token
    await until(async () => (await fixture.request(relay, computer, "/")) === "hello GET /");
    expect([connected, wait !== undefined && wait > 0, lapsed]).toEqual([
        "hello GET /",
        true,
        undefined,
    ]);
});

test("keep a name's destination at the edge until its machine answers it misdirected, then route to the machine serving it now", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort(), { destinations: new DestinationCache() });
    let isMoved = false;
    await fixture.tunnel(relay, {
        fetch: async () =>
            isMoved
                ? Response.json({ code: "MISDIRECTED_REQUEST", message: "moved" }, { status: 421 })
                : new Response("machine"),
    });
    const first = await fixture.request(relay, fixture.notes, "/");

    // move the space to the cloud machine while calling the kept machine
    await fixture.cloud("cloud");
    const placement = {
        id: fixture.spaceId,
        scope: ids.account,
        machine: fixture.machineId,
        epoch: 1,
    };
    await TestDirectory.place(fixture.accounts.database, placement, ids.machine);
    await TestDirectory.place(fixture.accounts.database, {
        ...placement,
        machine: ids.machine,
        epoch: 2,
    });
    await fixture.settle(relay);
    const kept = await fixture.request(relay, fixture.notes, "/");

    // follow the machine's misdirection to the cloud machine
    isMoved = true;
    expect([first, kept, await fixture.request(relay, fixture.notes, "/")]).toEqual([
        "machine",
        "machine",
        `cloud host="${fixture.notes}";proto=http`,
    ]);
});

test("follow a cloud machine's new endpoint", async () => {
    await using fixture = await RelayFixture.open(accounts, installation);
    const relay = await fixture.relay(await freePort());
    const cloud = `cloud-${fixture.space.slice(-12)}`;
    await fixture.cloud("first");
    await RelayFixture.place(fixture.accounts, cloud, ids.machine);
    await fixture.settle(relay);
    const name = `notes--${cloud}--acme.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`first host="${name}";proto=http`);

    // publish the cloud machine's new endpoint
    await fixture.cloud("second");

    await until(async () => (await fixture.request(relay, name, "/")).startsWith("second "));
});
