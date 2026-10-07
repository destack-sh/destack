import { afterAll, beforeAll, expect, test } from "@destack/test";
import { AccountFixture, ids } from "@destack/account/test";
import { RequestId } from "@destack/service/request";
import { DestinationCache } from "../src/server/index.ts";
import { TUNNEL_PROTOCOL, TunnelProtocol } from "../src/session/index.ts";
import { freePort, RelayFixture, type RelayWorkload, until } from "../src/test/index.ts";

/** The account service with each scenario's enrolled machine. */
let accounts: AccountFixture;

/** The relay's workload in the platform's region. */
let workload: RelayWorkload;

beforeAll(async () => {
    accounts = await AccountFixture.open();
    workload = await RelayFixture.workload(accounts);
});

afterAll(async () => {
    await accounts[Symbol.asyncDispose]();
});

test("forward a request for a space's name through its machine's tunnel, keeping its name, method and body", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // reach the machine's app by the space's name
    expect(
        await fixture.request(relay, fixture.notes, "/notes/1", { method: "POST", body: "pinned" }),
    ).toBe("hello POST /notes/1 pinned");
    expect([fixture.received, fixture.reports]).toEqual([[`POST ${fixture.notes}/notes/1`], []]);
});

test("reach a machine by its name within its account through its tunnel", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // reach the machine itself, and nothing by a name no standing machine of the account has
    expect([
        await fixture.request(relay, await fixture.computer(), "/status"),
        await fixture.request(relay, "missing.acme.destack.computer", "/status"),
    ]).toEqual(["hello GET /status", "404 NOT_FOUND"]);
});

test("tell a machine its name as its tunnel opens, and again at once after its rename", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay, { heartbeat: 60_000 });
    const before = await fixture.computer();
    await until(async () => fixture.names.length > 0);

    // rename the machine
    await accounts.machine(fixture.identity).machine.rename({
        accountId: ids.account,
        id: fixture.machineId,
        requestId: RequestId.create(),
        name: "renamed",
    });

    // call the machine by its new name once the relay tells it
    await until(async () => fixture.names.at(-1) === "renamed.acme.destack.computer");
    expect([
        fixture.names,
        await fixture.request(relay, "renamed.acme.destack.computer", "/status"),
        await fixture.request(relay, before, "/status"),
        fixture.reports,
    ]).toEqual([
        [before, "renamed.acme.destack.computer"],
        "hello GET /status",
        "404 NOT_FOUND",
        [],
    ]);
});

test("refuse names that lead nowhere, and a named machine that keeps no tunnel", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());

    // refuse other handles, spaces, shapes and domains, and the machine before it connects
    expect(
        await Promise.all(
            [
                `notes.${fixture.space}.rival.destack.space`,
                "notes.missing.acme.destack.space",
                `${fixture.space}.acme.destack.space`,
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
    await using fixture = await RelayFixture.open(accounts, workload);
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
    await using fixture = await RelayFixture.open(accounts, workload);
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
    await using fixture = await RelayFixture.open(accounts, workload);
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
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    const branch = `notes.feature-x--${fixture.space}.acme.destack.space`;
    expect([await fixture.request(relay, branch, "/"), fixture.received]).toEqual([
        "hello GET /",
        [`GET ${branch}/`],
    ]);
});

test("forward a name a region serves to the region's endpoint, keeping the host asked for", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    const cloud = `cloud-${fixture.space}`;
    await fixture.region("region");
    await RelayFixture.place(fixture.accounts, cloud, ids.region);
    await fixture.settle(relay);

    const name = `notes.${cloud}.acme.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`region host="${name}";proto=http`);
});

test("follow a space's move from its machine to a region", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // move the zone to the region at the next epoch
    await fixture.region("region");
    const zone = { id: fixture.spaceId, scope: ids.account, cell: fixture.machineId, epoch: 1 };
    await fixture.directory.move(zone, ids.region);
    await fixture.directory.place({ ...zone, cell: ids.region, epoch: 2 });

    await until(async () =>
        (await fixture.request(relay, fixture.notes, "/")).startsWith("region "),
    );
});

test("follow a space's rename", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // place the space under a new name, releasing the old one
    const renamed = `renamed-${fixture.space}`;
    await fixture.rename(renamed);

    // refuse the old name and reach the space by the new one
    await until(async () => (await fixture.request(relay, fixture.notes, "/")) !== "hello GET /");
    expect([
        await fixture.request(relay, fixture.notes, "/"),
        await fixture.request(relay, `notes.${renamed}.acme.destack.space`, "/"),
    ]).toEqual(["404 NOT_FOUND", "hello GET /"]);
});

test("resolve a handle from the relay's copy after the account service renames it", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // rename the account's handle as its owner, and rename it back afterwards for the other scenarios
    const owner = accounts.user(ids.owner);
    const rename = async (handle: string) => {
        const current = await owner.account.get({ scope: ids.owner, id: ids.account });
        await owner.account.update({
            scope: ids.owner,
            requestId: RequestId.create(),
            id: ids.account,
            revision: current.revision,
            handle,
        });
    };
    await rename("acme-renamed");
    try {
        // refuse the old handle and reach the space by the new one
        await until(
            async () => (await fixture.request(relay, fixture.notes, "/")) !== "hello GET /",
        );
        expect([
            await fixture.request(relay, fixture.notes, "/"),
            await fixture.request(relay, fixture.notes.replace(".acme.", ".acme-renamed."), "/"),
        ]).toEqual(["404 NOT_FOUND", "hello GET /"]);
    } finally {
        await rename("acme");
    }
});

test("follow a machine's revocation, closing its tunnel at once, refusing it again and recording the opening and the close", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay, { report: () => {} });
    const computer = await fixture.computer();
    expect(await fixture.request(relay, computer, "/")).toBe("hello GET /");

    // revoke the machine as its owner
    await accounts.user(ids.owner).machine.revoke({
        accountId: ids.account,
        id: fixture.machineId,
        requestId: RequestId.create(),
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
            call.execution.context.caller.type,
            call.execution.details,
        ]),
    ).toEqual([
        ["tunnel.open", ids.account, "subject", { name: computer }],
        ["tunnel.close", ids.account, "system", { reason: "machine-revoked" }],
    ]);
});

test("close a machine's tunnel once its last key is revoked, recording why", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
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
        await owner.key.revoke({ scope: ids.account, id: kept.id, requestId: RequestId.create() });
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

test("keep a name's destination at the edge until its cell answers it misdirected, then route to the cell serving it now", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort(), new DestinationCache());
    let isMoved = false;
    await fixture.tunnel(relay, {
        fetch: async () =>
            isMoved
                ? Response.json({ code: "MISDIRECTED_REQUEST", message: "moved" }, { status: 421 })
                : new Response("machine"),
    });
    const first = await fixture.request(relay, fixture.notes, "/");

    // move the zone to the region while reaching the kept machine
    await fixture.region("region");
    const zone = { id: fixture.spaceId, scope: ids.account, cell: fixture.machineId, epoch: 1 };
    await fixture.directory.move(zone, ids.region);
    await fixture.directory.place({ ...zone, cell: ids.region, epoch: 2 });
    await fixture.settle(relay);
    const kept = await fixture.request(relay, fixture.notes, "/");

    // follow the machine's misdirection to the region
    isMoved = true;
    expect([first, kept, await fixture.request(relay, fixture.notes, "/")]).toEqual([
        "machine",
        "machine",
        `region host="${fixture.notes}";proto=http`,
    ]);
});

test("follow a region's new endpoint", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    const cloud = `cloud-${fixture.space}`;
    await fixture.region("first");
    await RelayFixture.place(fixture.accounts, cloud, ids.region);
    await fixture.settle(relay);
    const name = `notes.${cloud}.acme.destack.space`;
    expect(await fixture.request(relay, name, "/")).toBe(`first host="${name}";proto=http`);

    // publish the region's new endpoint
    await fixture.region("second");

    await until(async () => (await fixture.request(relay, name, "/")).startsWith("second "));
});
