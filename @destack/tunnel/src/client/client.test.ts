import { testSimulation } from "@destack/test/world";
import { TestTelemetry } from "@destack/telemetry/test";
import { expect, refusal, test } from "@destack/test";
import { Simulation } from "@destack/environment/simulation";
import { schema } from "@destack/schema";
import { Frame, FrameFlag, FrameType, PING_MESSAGE, TUNNEL_PROTOCOL } from "../session/index.ts";
import { PackageId } from "@destack/package";
import { TUNNEL_METADATA_PATH, TunnelClient, type TunnelStatus } from "./client.ts";

/** The close code of a normal closure, from RFC 6455 section 7.4.1. */
const NORMAL_CLOSURE = 1000;

/** The tunnel URL of a relay where nothing listens. */
const NOWHERE = "http://relay.test/tunnel";

/** The relay's package, the audience of the tokens its tunnel accepts. */
const RELAY = PackageId.parse("package-01996ab0-0000-7000-8000-000000000002");

/** The machine dialing the tunnels. */
const MACHINE = schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000001");

/** A connection to a silent relay. */
interface SilentConnection {
    /** The offered subprotocols, as the client listed them. */
    readonly protocols: string | null;
    /** The messages the client sent after the upgrade, in order. */
    readonly messages: (string | Uint8Array)[];
    /** The close code the client sent, absent while open. */
    closed?: number;
}

/** Answer a request for the tunnel's metadata, naming the relay's package as its resource. */
function metadata(request: Request): Response | undefined {
    return new URL(request.url).pathname === TUNNEL_METADATA_PATH
        ? Response.json({ resource: PackageId.resource(RELAY) })
        : undefined;
}

/** Serve a relay that accepts upgrades and answers nothing after, recording each connection. */
function silentRelay(simulation: Simulation) {
    // accept each upgrade with the tunnel protocol, recording what the client sends
    const connections: SilentConnection[] = [];
    const listener = simulation.network.server("relay").http<SilentConnection>({
        hostname: "relay.test",
        port: 80,
        fetch: (request) => {
            // answer the metadata, and record each upgrade
            const answered = metadata(request);
            if (answered !== undefined) {
                return answered;
            }
            const connection: SilentConnection = {
                protocols: request.headers.get("sec-websocket-protocol"),
                messages: [],
            };
            connections.push(connection);

            return {
                attachment: connection,
                headers: { "sec-websocket-protocol": TUNNEL_PROTOCOL },
            };
        },
        websocket: {
            message: (socket, message) => socket.attachment.messages.push(message),
            close: (socket, code) => {
                socket.attachment.closed = code;
            },
        },
    });

    return { url: `${listener.url.origin}/tunnel`, connections };
}

/** Serve a relay that answers each session ping, dropping the first connections as it answers. */
function answeringRelay(simulation: Simulation, dropped = 0) {
    // accept each upgrade with the tunnel protocol, counting the connections
    const sockets = new Set<{ close: () => void }>();
    let connections = 0;
    const listener = simulation.network.server("relay").http<undefined>({
        hostname: "relay.test",
        port: 80,
        fetch: (request) => {
            // answer the metadata, and count each upgrade
            const answered = metadata(request);
            if (answered !== undefined) {
                return answered;
            }
            connections += 1;

            return {
                attachment: undefined,
                headers: { "sec-websocket-protocol": TUNNEL_PROTOCOL },
            };
        },
        websocket: {
            open: (socket) => void sockets.add(socket),
            message: (socket, message) => {
                // answer a session ping, closing the first connections in the same turn
                const frame = typeof message === "string" ? undefined : Frame.decode(message);
                if (frame?.type === FrameType.ping && frame.has(FrameFlag.syn)) {
                    socket.send(new Frame(FrameType.ping, FrameFlag.ack, 0, frame.value).encode());
                    if (connections <= dropped) {
                        socket.close();
                    }
                }
            },
            close: (socket) => void sockets.delete(socket),
        },
    });

    // drop every open connection
    const drop = () => {
        for (const socket of sockets) {
            socket.close();
        }
    };

    return { url: `${listener.url.origin}/tunnel`, connections: () => connections, drop };
}

/** Describe a warning the tunnel package recorded. */
function warning(message: string) {
    return {
        scope: "@destack/tunnel",
        event: "exception",
        severity: "WARN",
        attributes: { "exception.escaped": false, "exception.message": message },
    };
}

test("refuse callers waiting for a tunnel once the client closes", async () => {
    // dial a relay that never answers, as a machine with a token
    await using simulation = testSimulation();
    const statuses: TunnelStatus[] = [];
    const client = TunnelClient.open({
        url: NOWHERE,
        machineId: MACHINE,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        renewed: () => {},
        changed: (status) => void statuses.push(status),
        environment: simulation.environment("machine"),
        retry: { initialInterval: 10, maximumInterval: 10 },
    });
    const waiting = refusal(client.opened());

    // refuse the caller waiting before the close, and one asking after it, never telling it open
    await client.close();
    expect([await waiting, await refusal(client.opened()), statuses]).toEqual([
        ["SERVICE_UNAVAILABLE", "CLIENT_UNAVAILABLE", "tunnel client is unavailable: it is closed"],
        ["SERVICE_UNAVAILABLE", "CLIENT_UNAVAILABLE", "tunnel client is unavailable: it is closed"],
        [],
    ]);
});

test("dial again after the issuer grants no token for the relay's resource, warning of each failed dial", async () => {
    // dial with a token the issuer refuses, asked for the resource the relay names
    await using simulation = testSimulation();
    const statuses: TunnelStatus[] = [];
    const audiences: PackageId[] = [];
    using telemetry = new TestTelemetry(simulation.telemetry);
    const relay = silentRelay(simulation);
    const client = TunnelClient.open({
        url: relay.url,
        machineId: MACHINE,
        token: async (audience) => {
            audiences.push(audience);
            throw new TypeError("the issuer is unreachable");
        },
        fetch: async () => new Response(null, { status: 204 }),
        renewed: () => {},
        changed: (status) => void statuses.push(status),
        environment: simulation.environment("machine"),
        retry: { initialInterval: 10, maximumInterval: 10 },
    });

    // warn of two failed dials and stop cleanly
    await simulation.waitUntil(() => telemetry.records().length === 4);
    await client.close();
    const failed = [
        warning(`dialing ${relay.url} failed: TypeError: the issuer is unreachable`),
        warning(`tunnel to ${relay.url} did not open`),
    ];
    expect([telemetry.records(), statuses, audiences]).toEqual([
        [...failed, ...failed],
        [],
        [RELAY, RELAY],
    ]);
});

test("drop a tunnel whose relay stops answering, dialing again and closing without its close answered", async () => {
    // dial a relay that accepts the upgrade and answers nothing after
    await using simulation = testSimulation();
    const statuses: TunnelStatus[] = [];
    using telemetry = new TestTelemetry(simulation.telemetry);
    const relay = silentRelay(simulation);
    const client = TunnelClient.open({
        url: relay.url,
        machineId: MACHINE,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        renewed: () => {},
        changed: (status) => void statuses.push(status),
        environment: simulation.environment("machine"),
        heartbeat: 20,
        retry: { initialInterval: 10, maximumInterval: 10 },
    });

    // dial again after the unanswered liveness probe drops the socket, and close while connected to a peer that never answers the close
    await simulation.waitUntil(() => relay.connections.length >= 2);
    await client.close();
    await simulation.waitUntil(() => relay.connections[1]?.closed !== undefined);

    // offer the token, ping and probe before dropping the first connection, and ping on the second before the close
    const ping = new Frame(FrameType.ping, FrameFlag.syn, 0).encode();
    expect([relay.connections, telemetry.records(), statuses]).toEqual([
        [
            {
                protocols: `${TUNNEL_PROTOCOL}, destack.bearer.dG9rZW4`,
                messages: [ping, PING_MESSAGE],
                closed: NORMAL_CLOSURE,
            },
            {
                protocols: `${TUNNEL_PROTOCOL}, destack.bearer.dG9rZW4`,
                messages: [ping],
                closed: NORMAL_CLOSURE,
            },
        ],
        [warning(`tunnel to ${relay.url} did not open`)],
        [],
    ]);
});

test("tell the tunnel open once the relay answers, dialing once it drops, and nothing once the client closes", async () => {
    // dial a relay that answers
    await using simulation = testSimulation();
    const statuses: TunnelStatus[] = [];
    const relay = answeringRelay(simulation);
    const client = TunnelClient.open({
        url: relay.url,
        machineId: MACHINE,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        renewed: () => {},
        changed: (status) => void statuses.push(status),
        environment: simulation.environment("machine"),
        retry: { initialInterval: 10, maximumInterval: 10 },
    });

    // open the tunnel, drop it, open the next, then close the client
    await client.opened();
    relay.drop();
    await simulation.waitUntil(() => relay.connections() === 2 && statuses.length === 3);
    await client.close();
    expect(statuses).toEqual(["open", "dialing", "open"]);
});

test("tell no tunnel open whose relay drops it while answering", async () => {
    // dial a relay that drops the first connection as it answers
    await using simulation = testSimulation();
    const statuses: TunnelStatus[] = [];
    const relay = answeringRelay(simulation, 1);
    const client = TunnelClient.open({
        url: relay.url,
        machineId: MACHINE,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        renewed: () => {},
        changed: (status) => void statuses.push(status),
        environment: simulation.environment("machine"),
        retry: { initialInterval: 10, maximumInterval: 10 },
    });

    // open only the second tunnel
    await simulation.waitUntil(() => statuses.length === 1);
    const opened = [relay.connections(), statuses];
    await client.close();
    expect(opened).toEqual([2, ["open"]]);
});
