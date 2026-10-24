import { Failure } from "@destack/error";
import { DOMAINS } from "@destack/account/address";
import accountPackage from "@destack/account/package";
import { Snapshot } from "@destack/db";
import { RequestId } from "@destack/sync";
import { expect, refusal, test } from "@destack/test";
import relayPackage from "../src/package.ts";
import { TUNNEL_TOKEN_LIFETIME_MILLISECONDS, Relay, Tunnel } from "../src/server/index.ts";
import { type Renewal, TUNNEL_PROTOCOL, TunnelProtocol } from "../src/session/index.ts";
import { TestTunnelMachine } from "../src/test/index.ts";

/** The origin of the Europe residency's cloud machine. */
const EU_CLOUD = "https://eu.machine.test/";

/** The origin of the United States residency's cloud machine. */
const US_CLOUD = "https://us.machine.test/";

/** Describe the refusal of a name that resolves to nothing. */
function unresolved(name: string) {
    return ["NOT_FOUND", "NAME_UNRESOLVED", `${name} does not resolve`];
}

/** Describe the refusal of a machine no tunnel or endpoint reaches. */
function disconnected(computer: TestTunnelMachine) {
    return [
        "SERVICE_UNAVAILABLE",
        "MACHINE_DISCONNECTED",
        `machine ${computer.machine.id} is disconnected`,
    ];
}

/** Read a refusal's code, reason and message. */
async function refused(response: Response) {
    const { code, reason, message } = await Failure.read(response);

    return [code, reason, message];
}

/** Describe a tunnel event of the computer, with its reason for a close. */
function event(computer: TestTunnelMachine, name: string, reason?: string) {
    const machine = computer.machine.id;

    return [name, reason === undefined ? { machine } : { machine, reason }];
}

test("forward a request for a space's name through its machine's tunnel, preserving its name, method and body", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);

    // call the machine's app by the space's name
    expect([
        await computer.request(relay, computer.notes, "/notes/1", {
            method: "POST",
            body: "pinned",
        }),
        computer.received,
        computer.tunnels(),
    ]).toEqual([
        "hello POST /notes/1 pinned",
        [`POST ${computer.notes}/notes/1`],
        [event(computer, "tunnel.open")],
    ]);
});

test("forward a client's request to its machine with exactly the client's headers, its own authorization included", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);

    // call the space's app with the client's own credential and a header of its own
    const answered = await computer.request(relay, computer.notes, "/headers", {
        headers: { authorization: "Bearer client", "x-note": "1" },
    });
    expect([answered, computer.heads]).toEqual([
        "hello GET /headers",
        [
            [
                ["authorization", "Bearer client"],
                ["host", computer.notes],
                ["x-note", "1"],
            ],
        ],
    ]);
});

test("route a machine's name within its account through its tunnel", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);

    // call the machine itself, and nothing by a name no standing machine of the account has
    expect([
        await computer.request(relay, await computer.computer(), "/status"),
        await computer.request(relay, "missing--acme.destack.computer", "/status"),
    ]).toEqual(["hello GET /status", unresolved("missing--acme.destack.computer")]);
});

test("tell a machine its name as its tunnel opens, and again at once after its rename", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay, { heartbeat: 60_000 });
    const before = await computer.computer();

    // rename the machine as itself
    await computer.accounts().machine.rename({
        scope: computer.accountId,
        id: computer.machine.id,
        requestId: RequestId.create(computer.environment),
        name: "renamed",
    });

    // call the machine by its new name once the relay tells it
    const renamed = "renamed--acme.destack.computer";
    await computer.until(() => computer.names.at(-1) === renamed);
    await computer.settle(relay);
    expect([
        computer.names,
        await computer.request(relay, renamed, "/status"),
        await computer.request(relay, before, "/status"),
    ]).toEqual([[before, renamed], "hello GET /status", unresolved(before)]);
});

test("refuse names that lead nowhere, and a named machine that has no tunnel and whose endpoint does not answer", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();

    // refuse other handles, spaces, shapes and domains, and the machine before it connects
    const names = [
        "notes--work--rival.destack.space",
        "notes--missing--acme.destack.space",
        "work--acme.destack.space",
        "notes.work.acme.example.com",
    ];
    const answers = await Promise.all(names.map((name) => computer.request(relay, name, "/")));
    expect([...answers, await computer.request(relay, computer.notes, "/")]).toEqual([
        ...names.map(unresolved),
        ["BAD_GATEWAY", "ENDPOINT_UNREACHABLE", "https://computer.test/ does not answer"],
    ]);
});

test("refuse a forwarded request without its host, and one for a name another relay has no tunnel to", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.cloud("eu", "cloud", EU_CLOUD);
    await computer.place("site");
    await computer.settle(relay);

    // forward to the relay's own host with a Forwarded header lacking the host, then naming a cloud space
    const host = new URL(relay.origin).host;
    const site = "notes--site--acme.destack.space";
    expect([
        await computer.request(relay, host, "/", { headers: { forwarded: "for=192.0.2.1" } }),
        await computer.request(relay, host, "/", {
            headers: { forwarded: `host="${site}";proto=http` },
        }),
    ]).toEqual([
        ["BAD_REQUEST", "FORWARDED_HOST_MISSING", "the forwarded request has no host or protocol"],
        ["MISDIRECTED_REQUEST", "RELAY_OTHER", `${site} is tunnelled through another relay`],
    ]);
});

test("refuse the installation's own calls to a tunnel without a handshake or a client's head, and calls the tunnels and the root do not answer", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();

    // open and forward without their headers at the machine's partition, then call verbs neither answers
    const machine = computer.machine.id;
    const partition = Tunnel.partition(machine);
    const post = { method: "POST" };
    expect([
        await refused(await relay.itself(`/${machine}/open`, post, partition)),
        await refused(await relay.itself(`/${machine}/forward`, post, partition)),
        await refused(await relay.itself(`/${machine}/bogus`, post, partition)),
        await refused(await relay.itself(`/${machine}/bogus`, post)),
    ]).toEqual([
        ["BAD_REQUEST", "HANDSHAKE_MISSING", "the tunnel's opening has no handshake"],
        ["BAD_REQUEST", "FORWARDED_HEAD_MISSING", "the handed request has no client head"],
        ["NOT_FOUND", "TUNNEL_CALL_UNKNOWN", "unknown tunnel call bogus"],
        ["NOT_FOUND", "TUNNEL_CALL_UNKNOWN", "the relay's root answers no call bogus"],
    ]);
});

test("refuse a handshake with a person's token, of a machine left keyless, and of a revoked machine", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    const host = await computer.host("eu");
    const handshakes = new Relay({
        origin: relay.origin,
        issuer: host.universe.issuer,
        residency: "eu",
        itself: { url: relay.origin, fetch: async () => new Response(null) },
        view: Snapshot.live(computer.universe.storage.database),
        domains: DOMAINS,
        fetch: async () => new Response(null),
        synced: async () => {},
        environment: computer.environment,
    });
    const offer = (token: string) =>
        new Request(relay.url, {
            headers: { "sec-websocket-protocol": TunnelProtocol.offer(token).join(", ") },
        });
    const shake = (token: string) => refusal(handshakes.handshake(offer(token)));

    // offer the owner's token, then the machine's after its keys' revoke, then after its own
    const person = await computer.owner.exchange(relayPackage.id);
    const machine = await computer.machine.token(relayPackage.id);
    const accounts = computer.owner.client(accountPackage);
    const { items } = await accounts.key.list({
        scope: computer.accountId,
        where: { parentId: computer.machine.id },
    });
    for (const key of items) {
        await accounts.key.revoke({
            scope: computer.accountId,
            id: key.id,
            requestId: RequestId.create(computer.environment),
        });
    }
    const keyless = await shake(machine);
    await accounts.machine.revoke({
        scope: computer.accountId,
        id: computer.machine.id,
        requestId: RequestId.create(computer.environment),
    });

    expect([await shake(person.accessToken), keyless, await shake(machine)]).toEqual([
        ["FORBIDDEN", "MACHINE_REQUIRED", "only machines can open tunnels"],
        ["UNAUTHORIZED", "MACHINE_UNKEYED", `machine ${computer.machine.id} has no key`],
        ["UNAUTHORIZED", "MACHINE_REVOKED", `machine ${computer.machine.id} is revoked`],
    ]);
});

test("publish a machine's tunnel as the relay accepting it, at the machine's name and its token's lapse", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    const renewals: Renewal[] = [];
    await computer.tunnel(relay, { renewed: (renewal) => void renewals.push(renewal) });

    // keep the endpoint the relay published, at the name and lapse it told the machine
    const published = await computer.endpoint();
    const [renewal] = renewals;
    const name = await computer.computer();
    expect([renewals.length, published?.url, published?.relay]).toEqual([
        1,
        `https://${name}`,
        { url: new URL(relay.url).origin, lapsesAt: renewal?.lapsesAt },
    ]);
});

test("refuse a machine's handshake at a relay of another residency than its account's", async () => {
    // offer the eu machine's token to the us relay
    await using computer = await TestTunnelMachine.open();
    const us = await computer.relay({ residency: "us" });
    const host = await computer.host("us");
    const handshakes = new Relay({
        origin: us.origin,
        issuer: host.universe.issuer,
        residency: "us",
        itself: { url: us.origin, fetch: async () => new Response(null) },
        view: Snapshot.live(computer.universe.storage.database),
        domains: DOMAINS,
        fetch: async () => new Response(null),
        synced: async () => {},
        environment: computer.environment,
    });
    const token = await computer.machine.token(relayPackage.id);
    const offer = new Request(us.url, {
        headers: { "sec-websocket-protocol": TunnelProtocol.offer(token).join(", ") },
    });

    expect(await refusal(handshakes.handshake(offer))).toEqual([
        "FORBIDDEN",
        "RESIDENCY_FOREIGN",
        `machine ${computer.machine.id} tunnels through the relay of its account's residency`,
    ]);
});

test("leave a machine revoked elsewhere alone, starting no tunnel partition for it", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);

    // enroll and revoke another machine of the account, which tunnels nowhere
    const other = await computer.universe.enroll(
        computer.owner,
        computer.accountId,
        "https://other.test/",
    );
    await computer.owner.client(accountPackage).machine.revoke({
        scope: computer.accountId,
        id: other.id,
        requestId: RequestId.create(computer.environment),
    });
    await computer.settle(relay);

    // run the root and the computer's partition alone
    expect(relay.partitions).toEqual([undefined, Tunnel.partition(computer.machine.id)]);
});

test("refuse a tunnel without a valid machine token among its subprotocols", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();

    // offer a forged token, a valid one only in the authorization header, and one without the tunnel protocol
    const upgrade = (headers: Record<string, string>) =>
        computer.simulation.network
            .of("computer")
            .fetch(
                new Request(relay.url, {
                    headers: { upgrade: "websocket", connection: "upgrade", ...headers },
                }),
            )
            .then((response) => response.status);
    const token = await computer.machine.token(relayPackage.id);
    const [, bearer = ""] = TunnelProtocol.offer(token);
    expect([
        await upgrade({ "sec-websocket-protocol": TunnelProtocol.offer("forged").join(", ") }),
        await upgrade({ authorization: `Bearer ${token}` }),
        await upgrade({ "sec-websocket-protocol": bearer }),
    ]).toEqual([401, 401, 401]);
});

test("open a tunnel with the token offered as a subprotocol, answering the tunnel protocol only", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();

    // open a WebSocket as the machine, and read the protocol the relay chose
    const offered = TunnelProtocol.offer(await computer.machine.token(relayPackage.id));
    const socket = computer.simulation.network
        .of("computer")
        .websocket(relay.url.replace("http:", "ws:"), offered);
    const protocol = await new Promise<string>((resolve, reject) => {
        socket.addEventListener("open", () => resolve(socket.protocol));
        socket.addEventListener("close", () => reject(new Error("the relay refused the tunnel")));
    });
    socket.close();
    expect(protocol).toBe(TUNNEL_PROTOCOL);
});

test("restore the route when the machine reconnects after its relay restarts", async () => {
    await using computer = await TestTunnelMachine.open();
    const first = await computer.relay();
    await computer.tunnel(first);

    // restart the relay at the same origin, and wait for the machine to dial it again
    await computer.stop(first);
    const second = await computer.relay({ installation: first.installation });
    await computer.until(
        async () =>
            (await computer.request(second, computer.notes, "/again")) === "hello GET /again",
    );
    expect(computer.tunnels()).toEqual([
        event(computer, "tunnel.open"),
        event(computer, "tunnel.close", "closed"),
        event(computer, "tunnel.open"),
    ]);
});

test("route a branch to its space's machine under the branch's name", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);

    const branch = "notes--feature-x---work--acme.destack.space";
    expect([await computer.request(relay, branch, "/"), computer.received]).toEqual([
        "hello GET /",
        [`GET ${branch}/`],
    ]);
});

test("forward a name a cloud machine serves to the machine's endpoint, preserving the host asked for", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.cloud("eu", "cloud", EU_CLOUD);
    await computer.place("site");
    await computer.settle(relay);

    const name = "notes--site--acme.destack.space";
    expect(await computer.request(relay, name, "/")).toBe(`cloud host="${name}";proto=http`);
});

test("route the name of a space whose account lives in another residency through the directory", async () => {
    // place a space of a us account on the us cloud machine, and route its name from the eu relay
    await using computer = await TestTunnelMachine.open("us");
    const relay = await computer.relay();
    await computer.cloud("us", "us", US_CLOUD);
    await computer.place("site");
    await computer.settle(relay);

    const name = "notes--site--acme.destack.space";
    expect(await computer.request(relay, name, "/")).toBe(`us host="${name}";proto=http`);
});

test("route a request from an eu relay to a space on a personal machine tunnelled to a us relay", async () => {
    // tunnel a us account's machine to the us relay, and start the eu relay
    await using computer = await TestTunnelMachine.open("us");
    const us = await computer.relay({ residency: "us" });
    await computer.tunnel(us);
    const eu = await computer.relay();

    // forward the space's name from the eu relay through the us relay to the machine
    expect([await computer.request(eu, computer.notes, "/eu?x=1"), computer.received]).toEqual([
        "hello GET /eu",
        [`GET ${computer.notes}/eu`],
    ]);
});

test("route a request from an eu relay to a machine's name in the us residency through the us relay, until its tunnel lapses", async () => {
    // tunnel a us account's machine to the us relay, offering its first token alone, and start the eu relay
    await using computer = await TestTunnelMachine.open("us");
    const us = await computer.relay({ residency: "us" });
    const first = await computer.machine.token(relayPackage.id);
    await computer.tunnel(us, { token: async () => first, heartbeat: 60_000 });
    const eu = await computer.relay();
    const name = await computer.computer();
    const forwarded = await computer.request(eu, name, "/status");

    // refuse the name at the eu relay once the published tunnel lapsed, and end the socket at the us relay
    await computer.simulation.run({
        to: computer.clock.now() + TUNNEL_TOKEN_LIFETIME_MILLISECONDS,
    });
    const lapsed = await computer.request(eu, name, "/status");
    await computer.until(() => computer.tunnels().length === 2);
    expect([forwarded, lapsed, computer.received, computer.tunnels()]).toEqual([
        "hello GET /status",
        disconnected(computer),
        [`GET ${name}/status`],
        [event(computer, "tunnel.open"), event(computer, "tunnel.close", "lapsed")],
    ]);
});

test("follow a space's rename", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);
    const before = await computer.request(relay, computer.notes, "/");

    // place the space under a new name, releasing the old one
    await computer.owner.client(accountPackage).space.rename({
        accountId: computer.accountId,
        id: computer.spaceId,
        requestId: RequestId.create(computer.environment),
        name: "renamed",
    });
    await computer.settle(relay);

    // refuse the old name and call the space by the new one
    const renamed = "notes--renamed--acme.destack.space";
    expect([
        before,
        await computer.request(relay, computer.notes, "/"),
        await computer.request(relay, renamed, "/"),
    ]).toEqual(["hello GET /", unresolved(computer.notes), "hello GET /"]);
});

test("resolve a handle from the relay's replica after the account renames it", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);
    const before = await computer.request(relay, computer.notes, "/");

    // rename the account's handle as its owner
    const accounts = computer.owner.client(accountPackage);
    const target = { scope: computer.owner.id, id: computer.accountId };
    const current = await accounts.account.get(target);
    await accounts.account.update({
        ...target,
        requestId: RequestId.create(computer.environment),
        revision: current.revision,
        handle: "acme-renamed",
    });
    await computer.settle(relay);

    // refuse the old handle and call the space by the new one
    const renamed = "notes--work--acme-renamed.destack.space";
    expect([
        before,
        await computer.request(relay, computer.notes, "/"),
        await computer.request(relay, renamed, "/"),
    ]).toEqual(["hello GET /", unresolved(computer.notes), "hello GET /"]);
});

test("end a revoked machine's tunnel at once, refuse its names, and emit the opening and the close", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);
    const name = await computer.computer();
    const before = await computer.request(relay, name, "/");

    // revoke the machine as its owner, and wait for its tunnel to end
    await computer.owner.client(accountPackage).machine.revoke({
        scope: computer.accountId,
        id: computer.machine.id,
        requestId: RequestId.create(computer.environment),
    });
    await computer.until(() => computer.tunnels().length === 2);
    await computer.settle(relay);

    // refuse its name and its space's name, recording why its tunnel ended
    expect([
        before,
        await computer.request(relay, name, "/"),
        await computer.request(relay, computer.notes, "/"),
        computer.tunnels(),
    ]).toEqual([
        "hello GET /",
        unresolved(name),
        disconnected(computer),
        [event(computer, "tunnel.open"), event(computer, "tunnel.close", "machine-revoked")],
    ]);
});

test("end a tunnel at once as its relay restarts, when its machine was revoked meanwhile", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);
    const before = await computer.request(relay, computer.notes, "/");

    // revoke the machine as its owner while the relay's instances are stopped
    let isRestarted = false;
    const restarting = relay.restart(async () => {
        await computer.owner.client(accountPackage).machine.revoke({
            scope: computer.accountId,
            id: computer.machine.id,
            requestId: RequestId.create(computer.environment),
        });
    });
    void restarting.then(() => {
        isRestarted = true;
    });
    await computer.until(() => isRestarted);

    // end the tunnel as its partition starts again
    await computer.until(() => computer.tunnels().length === 2);
    expect([before, computer.tunnels()]).toEqual([
        "hello GET /",
        [event(computer, "tunnel.open"), event(computer, "tunnel.close", "machine-revoked")],
    ]);
});

test("end a machine's tunnel once its last key is revoked, emitting why", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);
    const before = await computer.request(relay, computer.notes, "/");

    // revoke the machine's keys as its owner
    const accounts = computer.owner.client(accountPackage);
    const { items } = await accounts.key.list({
        scope: computer.accountId,
        where: { parentId: computer.machine.id },
    });
    for (const key of items) {
        await accounts.key.revoke({
            scope: computer.accountId,
            id: key.id,
            requestId: RequestId.create(computer.environment),
        });
    }

    // end the tunnel at once, emitting the revoked key as the reason
    await computer.until(() => computer.tunnels().length === 2);
    expect([
        before,
        await computer.request(relay, computer.notes, "/"),
        computer.tunnels(),
    ]).toEqual([
        "hello GET /",
        disconnected(computer),
        [event(computer, "tunnel.open"), event(computer, "tunnel.close", "key-revoked")],
    ]);
});

test("end a disabled machine's tunnel, and route to it again once it is enabled and reconnects", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);
    const before = await computer.request(relay, computer.notes, "/");

    // disable the machine as its owner, suspending its keys and ending its tunnel
    const accounts = computer.owner.client(accountPackage);
    const target = { scope: computer.accountId, id: computer.machine.id };
    await accounts.machine.disable({
        ...target,
        requestId: RequestId.create(computer.environment),
    });
    await computer.until(() => computer.tunnels().length === 2);

    // enable it again, and route to it once it reconnects through its tunnel
    await accounts.machine.enable({ ...target, requestId: RequestId.create(computer.environment) });
    await computer.until(
        async () => (await computer.request(relay, computer.notes, "/")) === "hello GET /",
    );
    expect([before, computer.tunnels()]).toEqual([
        "hello GET /",
        [
            event(computer, "tunnel.open"),
            event(computer, "tunnel.close", "key-revoked"),
            event(computer, "tunnel.open"),
        ],
    ]);
});

test("end a machine's socket once its token lapses, and route to the machine again once it reconnects", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();

    // offer the first token alone until told otherwise, so the machine cannot renew before the lapse
    const first = await computer.machine.token(relayPackage.id);
    let isRenewable = false;
    await computer.tunnel(relay, {
        token: () =>
            isRenewable ? computer.machine.token(relayPackage.id) : Promise.resolve(first),
        heartbeat: 60_000,
    });
    const before = await computer.request(relay, computer.notes, "/");

    // end the socket past its lapse
    await computer.simulation.run({
        to: computer.clock.now() + TUNNEL_TOKEN_LIFETIME_MILLISECONDS,
    });
    await computer.until(() => computer.tunnels().length === 2);

    // route again once the machine reconnects with a new token
    isRenewable = true;
    await computer.until(
        async () => (await computer.request(relay, computer.notes, "/")) === "hello GET /",
    );
    expect([before, computer.tunnels()]).toEqual([
        "hello GET /",
        [
            event(computer, "tunnel.open"),
            event(computer, "tunnel.close", "lapsed"),
            event(computer, "tunnel.open"),
        ],
    ]);
});

test("close a machine's tunnel as moved at a relay its endpoint no longer leads to", async () => {
    // reconcile the machine as another relay of the residency, recording its calls
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);
    const host = await computer.host("eu");
    const told: [string, unknown][] = [];
    const elsewhere = new Relay({
        origin: "http://other.relay.test",
        issuer: host.universe.issuer,
        residency: "eu",
        itself: {
            url: "http://other.relay.test",
            fetch: async (request) => {
                told.push([
                    new URL(request.url).pathname.split("/").at(-1) ?? "",
                    await request.json(),
                ]);

                return new Response(null);
            },
        },
        view: Snapshot.live(computer.universe.storage.database),
        domains: DOMAINS,
        fetch: async () => new Response(null),
        synced: async () => {},
        environment: computer.environment,
    });
    await elsewhere.reconcile(computer.machine.id);

    expect(told).toEqual([["close", { reason: "moved" }]]);
});

test("follow a cloud machine's new endpoint", async () => {
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.cloud("eu", "first", EU_CLOUD);
    await computer.place("site");
    await computer.settle(relay);
    const name = "notes--site--acme.destack.space";
    const before = await computer.request(relay, name, "/");

    // publish the cloud machine's new endpoint
    await computer.cloud("eu", "second", "https://second.machine.test/");
    await computer.settle(relay);
    expect([before, await computer.request(relay, name, "/")]).toEqual([
        `first host="${name}";proto=http`,
        `second host="${name}";proto=http`,
    ]);
});
