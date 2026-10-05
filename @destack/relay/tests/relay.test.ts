import { afterAll, beforeAll, expect, test } from "@destack/test";
import { AccountFixture, ids } from "@destack/host/test";
import { RequestId } from "@destack/service/request";
import { TUNNEL_PROTOCOL, TunnelProtocol } from "../src/session/index.ts";
import { freePort, RelayFixture, type RelayWorkload, until } from "../src/test/index.ts";

/** The account service with each scenario's enrolled host. */
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

test("forward a request for a space's name through its host's tunnel, keeping its name, method and body", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // reach the host's app by the space's name
    expect(
        await fixture.request(relay, fixture.notes, "/notes/1", { method: "POST", body: "pinned" }),
    ).toBe("hello POST /notes/1 pinned");
    expect([fixture.received, fixture.reports]).toEqual([[`POST ${fixture.notes}/notes/1`], []]);
});

test("reach a host by its name within its account through its tunnel", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // reach the host itself, and nothing by a name no standing host of the account has
    expect([
        await fixture.request(relay, await fixture.computer(), "/status"),
        await fixture.request(relay, "missing.acme.destack.computer", "/status"),
    ]).toEqual(["hello GET /status", "404 NOT_FOUND"]);
});

test("tell a host its name as its tunnel opens, and again at once after its rename", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay, { heartbeat: 60_000 });
    const before = await fixture.computer();
    await until(async () => fixture.names.length > 0);

    // rename the host
    await accounts.host(fixture.identity).host.rename({
        accountId: ids.account,
        id: fixture.hostId,
        requestId: RequestId.create(),
        name: "renamed",
    });

    // call the host by its new name once the relay tells it
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

test("refuse names that lead nowhere, and a named host that keeps no tunnel", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());

    // refuse other handles, spaces, shapes and domains, and the host before it connects
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

test("refuse a tunnel without a valid host token among its subprotocols", async () => {
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

    // open a WebSocket as the host, and read the protocol the relay chose
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

test("keep the route while the host renews its tunnel's token", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);

    // outlive several renewals without a failure
    await new Promise((resolve) => {
        setTimeout(resolve, 500);
    });
    expect([await fixture.request(relay, fixture.notes, "/"), fixture.reports]).toEqual([
        "hello GET /",
        [],
    ]);
});

test("restore the route when the host reconnects after its relay restarts", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const port = await freePort();
    const first = await fixture.relay(port);
    await fixture.tunnel(first);

    // restart the relay on the same port, and wait for the host to dial it again
    await fixture.stop(first);
    const second = await fixture.relay(port);
    await until(
        async () => (await fixture.request(second, fixture.notes, "/again")) === "hello GET /again",
    );
});

test("route a branch to its space's host under the branch's name", async () => {
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

test("follow a space's move from its host to a region", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);
    expect(await fixture.request(relay, fixture.notes, "/")).toBe("hello GET /");

    // move the zone to the region at the next epoch
    await fixture.region("region");
    const zone = { id: fixture.spaceId, scope: ids.account, cell: fixture.hostId, epoch: 1 };
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

test("follow a host's revocation", async () => {
    await using fixture = await RelayFixture.open(accounts, workload);
    const relay = await fixture.relay(await freePort());
    await fixture.tunnel(relay);
    const computer = await fixture.computer();
    expect(await fixture.request(relay, computer, "/")).toBe("hello GET /");

    // revoke the host as its owner
    await accounts.user(ids.owner).host.revoke({
        accountId: ids.account,
        id: fixture.hostId,
        requestId: RequestId.create(),
    });

    await until(async () => (await fixture.request(relay, computer, "/")) === "404 NOT_FOUND");
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
