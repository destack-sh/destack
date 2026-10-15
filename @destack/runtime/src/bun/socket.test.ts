import { expect, onTestFinished, test } from "@destack/test";
import { TestTelemetry } from "@destack/telemetry/test";
import { BunHibernation } from "./socket.ts";
import { SOCKET_HEADER, SOCKET_INSTANCE_HEADER } from "../workload/socket.ts";

/** The instance holding the test's socket. */
const INSTANCE = "instance-test";

test("report a socket close its instance refuses, delivering the close once", async () => {
    using telemetry = TestTelemetry.register();

    // refuse every close, as an instance whose replicas are stale answers 503
    const delivered: string[] = [];
    const hibernation = new BunHibernation(async (_instanceId, path, request) => {
        delivered.push(`${request.method} ${path}`);

        return new Response(null, { status: request.method === "DELETE" ? 503 : 204 });
    });

    // hold a socket the instance accepts under one identifier
    const server = Bun.serve({
        port: 0,
        fetch: (request, served) =>
            hibernation.upgrade(
                served,
                request,
                new Response(null, {
                    headers: {
                        [SOCKET_HEADER]: JSON.stringify({ id: "socket-1", attachment: null }),
                        [SOCKET_INSTANCE_HEADER]: INSTANCE,
                    },
                }),
            )
                ? undefined
                : new Response(null, { status: 400 }),
        websocket: hibernation.websocket,
    });
    onTestFinished(() => server.stop(true));

    // open and close the socket from the client
    const socket = new WebSocket(`ws://127.0.0.1:${server.port}/`);
    await new Promise((resolve) => {
        socket.addEventListener("open", resolve, { once: true });
    });
    socket.close();
    await expect.poll(() => delivered.length).toBe(1);

    // capture the refused close once
    await expect.poll(() => telemetry.exceptions().length).toBe(1);
    expect([delivered, telemetry.take()]).toEqual([
        ["DELETE /.destack/socket/socket-1"],
        [
            {
                scope: "@destack/runtime",
                name: "Error",
                message: `instance ${INSTANCE} refused the close of socket socket-1: 503`,
            },
        ],
    ]);
});
