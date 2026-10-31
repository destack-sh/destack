import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Simulation } from "@destack/environment/simulation";
import { TestTelemetry } from "@destack/telemetry/test";
import { HibernationHost, SOCKET_HEADER, SOCKET_INSTANCE_HEADER } from "./socket.ts";

/** The instance of the test's socket. */
const INSTANCE = "instance-test";

/** List the descriptions of the test instance's hibernated sockets. */
async function hibernated(host: HibernationHost): Promise<unknown[]> {
    const listed = await host.answer(INSTANCE, new Request("http://host.test/.destack/socket"));

    return schema.array(schema.unknown()).parse(await listed.json());
}

test("report a socket close its instance refuses, delivering the close once", async () => {
    // refuse every close, as an instance whose replicas are stale answers 503
    const simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 16) });
    using telemetry = new TestTelemetry(simulation.telemetry);
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
        fetch: (request, listener) =>
            host.upgrade(listener, request, accepted)
                ? undefined
                : new Response(null, { status: 400 }),
        websocket: host.websocket,
    });

    // open and close the socket from the client
    const socket = simulation.network.of("client").websocket("ws://host.test/");
    await simulation.until(() => socket.readyState === WebSocket.OPEN);
    socket.close();
    await simulation.until(() => telemetry.exceptions().length === 1);

    // deliver the close once and capture its refusal once
    expect([delivered, telemetry.take()]).toEqual([
        ["DELETE /.destack/socket/socket-1"],
        [
            {
                scope: "@destack/object",
                name: "FamilyError",
                message: `instance ${INSTANCE} rejected closing socket socket-1: 503`,
            },
        ],
    ]);
});

test("drop an instance's send, store and close to a socket once its client closed it, as a WebSocket discards them", async () => {
    // hibernate a socket the instance accepts, then close it from the client
    const simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 16) });
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
        fetch: (request, listener) =>
            host.upgrade(listener, request, accepted)
                ? undefined
                : new Response(null, { status: 400 }),
        websocket: host.websocket,
    });
    const socket = simulation.network.of("client").websocket("ws://host.test/");
    await simulation.until(() => socket.readyState === WebSocket.OPEN);
    socket.close();
    await simulation.until(async () => (await hibernated(host)).length === 0);

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
