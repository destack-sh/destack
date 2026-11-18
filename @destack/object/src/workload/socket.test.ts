import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Simulation } from "@destack/environment/simulation";
import { TestTelemetry } from "@destack/telemetry/test";
import { CallError } from "../error/call.ts";
import { HibernationHost, SOCKET_HEADER, SOCKET_INSTANCE_HEADER } from "./socket.ts";
import { testWorldOptions } from "@destack/test/world";

/** The instance of the test's socket. */
const INSTANCE = "instance-test";

/** List the descriptions of the test instance's hibernated sockets. */
async function hibernated(host: HibernationHost): Promise<unknown[]> {
    const listed = await host.answer(INSTANCE, new Request("http://host.test/.destack/socket"));

    return schema.array(schema.unknown()).parse(await listed.json());
}

test("state a refused upgrade on its socket: the refusal's body as one frame, then a policy close", async () => {
    // refuse every upgrade with a declared failure, as a gateway answers
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 16),
    });
    const refused = new CallError("NOT_FOUND", {
        reason: "SCOPE_NOT_FOUND",
        domain: "@destack/object",
        message: "scope 'space-1' not found",
    });
    const host = new HibernationHost(
        async () => new Response(null, { status: 204 }),
        simulation.environment("host").telemetry,
    );
    simulation.network.server("host").http({
        hostname: "host.test",
        port: 80,
        ...host.listen(async () => Response.json(refused.toJSON(), { status: refused.status })),
    });

    // open a socket, recording its frames and its close
    const socket = simulation.network.of("client").websocket("ws://host.test/", ["destack.store"]);
    const frames: unknown[] = [];
    socket.addEventListener("message", (event) => frames.push(JSON.parse(String(event.data))));
    const closed = new Promise<CloseEvent>((resolve) => {
        socket.addEventListener("close", resolve, { once: true });
    });
    const { code, reason } = await simulation.wait(closed);

    // expect the refusal's body once, no subprotocol, and a policy close naming its reason
    expect({ protocol: socket.protocol, frames, code, reason }).toEqual({
        protocol: "",
        frames: [refused.toJSON()],
        code: 1008,
        reason: "upgrade refused: SCOPE_NOT_FOUND",
    });
});

test("report a socket close its instance refuses, delivering the close once", async () => {
    // refuse every close, as an instance whose replicas are stale answers 503
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 16),
    });
    using telemetry = TestTelemetry.of(simulation.telemetry);
    const delivered: string[] = [];
    const host = new HibernationHost(async (_instanceId, path, request) => {
        delivered.push(`${request.method} ${path}`);

        return new Response(null, { status: request.method === "DELETE" ? 503 : 204 });
    }, simulation.environment("host").telemetry);

    // hibernate a socket the instance accepts under one identifier
    const accepted = new Response(null, {
        headers: {
            [SOCKET_HEADER]: JSON.stringify({ id: "socket-1", attachment: null }),
            [SOCKET_INSTANCE_HEADER]: INSTANCE,
        },
    });
    simulation.network.server("host").http({
        hostname: "host.test",
        port: 80,
        fetch: () => host.accept(accepted),
        websocket: host.websocket,
    });

    // open and close the socket from the client
    const socket = simulation.network.of("client").websocket("ws://host.test/");
    await simulation.waitUntil(() => socket.readyState === WebSocket.OPEN);
    socket.close();
    await simulation.waitUntil(() => telemetry.exceptions().length === 1);

    // deliver the opening, then the close once, and capture its refusal once
    expect([delivered, telemetry.take()]).toEqual([
        ["PUT /.destack/socket/socket-1", "DELETE /.destack/socket/socket-1"],
        [
            {
                scope: "@destack/object",
                name: "FamilyError",
                message: `could not close socket 'socket-1' on instance '${INSTANCE}': status 503`,
            },
        ],
    ]);
});

test("drop an instance's send, store and close to a socket once its client closed it, as a WebSocket discards them", async () => {
    // hibernate a socket the instance accepts, then close it from the client
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 16),
    });
    const host = new HibernationHost(
        async () => new Response(null, { status: 204 }),
        simulation.environment("host").telemetry,
    );
    const accepted = new Response(null, {
        headers: {
            [SOCKET_HEADER]: JSON.stringify({ id: "socket-1", attachment: null }),
            [SOCKET_INSTANCE_HEADER]: INSTANCE,
        },
    });
    simulation.network.server("host").http({
        hostname: "host.test",
        port: 80,
        fetch: () => host.accept(accepted),
        websocket: host.websocket,
    });
    const socket = simulation.network.of("client").websocket("ws://host.test/");
    await simulation.waitUntil(() => socket.readyState === WebSocket.OPEN);
    socket.close();
    await simulation.waitUntil(async () => (await hibernated(host)).length === 0);

    // answer each call to the closed socket as done, leaving no socket behind
    const url = "http://host.test/.destack/socket/socket-1";
    const answers = [
        await host.answer(INSTANCE, new Request(url, { method: "POST", body: "late" })),
        await host.answer(INSTANCE, new Request(url, { method: "PUT", body: "{}" })),
        await host.answer(INSTANCE, new Request(url, { method: "DELETE" })),
    ];
    expect([answers.map((answer) => answer.status), await hibernated(host)]).toEqual([
        [204, 204, 204],
        [],
    ]);
});
