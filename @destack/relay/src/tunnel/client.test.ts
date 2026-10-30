import { testSimulation } from "@destack/test/world";
import { TestTelemetry } from "@destack/telemetry/test";
import { expect, refusal, test } from "@destack/test";
import { Simulation } from "@destack/environment/simulation";
import { schema } from "@destack/schema";
import { Frame, FrameFlag, FrameType, PING_MESSAGE, TUNNEL_PROTOCOL } from "../session/index.ts";
import { TunnelClient } from "./client.ts";

/** The close code of a normal closure, from RFC 6455 section 7.4.1. */
const NORMAL_CLOSURE = 1000;

/** The tunnel URL of a relay where nothing listens. */
const NOWHERE = "http://relay.test/tunnel";

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

/** Serve a relay that accepts upgrades and answers nothing after, recording each connection. */
function silentRelay(simulation: Simulation) {
    // accept each upgrade with the tunnel protocol, recording what the client sends
    const connections: SilentConnection[] = [];
    const listener = simulation.network.server("relay").http<SilentConnection>({
        hostname: "relay.test",
        port: 80,
        fetch: (request, listening) => {
            const connection: SilentConnection = {
                protocols: request.headers.get("sec-websocket-protocol"),
                messages: [],
            };
            connections.push(connection);
            listening.upgrade(request, {
                attachment: connection,
                headers: { "sec-websocket-protocol": TUNNEL_PROTOCOL },
            });

            return undefined;
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

/** Describe a warning the relay package recorded. */
function warning(message: string) {
    return {
        scope: "@destack/relay",
        event: "exception",
        severity: "WARN",
        attributes: { "exception.escaped": false, "exception.message": message },
    };
}

test("refuse callers waiting for a tunnel once the client closes", async () => {
    // dial a relay that never answers, as a machine with a token
    await using simulation = testSimulation();
    const client = TunnelClient.open({
        url: NOWHERE,
        machineId: MACHINE,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        renewed: () => {},
        environment: simulation.environment("machine"),
        retry: { initialInterval: 10, maximumInterval: 10 },
    });
    const waiting = refusal(client.opened());

    // refuse the caller waiting before the close, and one asking after it
    await client.close();
    expect([await waiting, await refusal(client.opened())]).toEqual([
        ["SERVICE_UNAVAILABLE", "CLIENT_CLOSED", "tunnel client is closed"],
        ["SERVICE_UNAVAILABLE", "CLIENT_CLOSED", "tunnel client is closed"],
    ]);
});

test("dial again after the issuer grants no token, warning of each failed dial", async () => {
    // dial with a token the issuer refuses
    await using simulation = testSimulation();
    using telemetry = new TestTelemetry(simulation.telemetry);
    const client = TunnelClient.open({
        url: NOWHERE,
        machineId: MACHINE,
        token: async () => {
            throw new TypeError("the issuer is unreachable");
        },
        fetch: async () => new Response(null, { status: 204 }),
        renewed: () => {},
        environment: simulation.environment("machine"),
        retry: { initialInterval: 10, maximumInterval: 10 },
    });

    // warn of two failed dials and stop cleanly
    await simulation.until(() => telemetry.records().length === 4);
    await client.close();
    const failed = [
        warning(`dialing ${NOWHERE} failed: TypeError: the issuer is unreachable`),
        warning(`tunnel to ${NOWHERE} did not open`),
    ];
    expect(telemetry.records()).toEqual([...failed, ...failed]);
});

test("drop a tunnel whose relay stops answering, dialing again and closing without its close answered", async () => {
    // dial a relay that accepts the upgrade and answers nothing after
    await using simulation = testSimulation();
    using telemetry = new TestTelemetry(simulation.telemetry);
    const relay = silentRelay(simulation);
    const client = TunnelClient.open({
        url: relay.url,
        machineId: MACHINE,
        token: async () => "token",
        fetch: async () => new Response(null, { status: 204 }),
        renewed: () => {},
        environment: simulation.environment("machine"),
        heartbeat: 20,
        retry: { initialInterval: 10, maximumInterval: 10 },
    });

    // dial again after the unanswered liveness probe drops the socket, and close while connected to a peer that never answers the close
    await simulation.until(() => relay.connections.length >= 2);
    await client.close();
    await simulation.until(() => relay.connections[1]?.closed !== undefined);

    // offer the token, ping and probe before dropping the first connection, and ping on the second before the close
    const ping = new Frame(FrameType.ping, FrameFlag.syn, 0).encode();
    expect([relay.connections, telemetry.records()]).toEqual([
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
    ]);
});
