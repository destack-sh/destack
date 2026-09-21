import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Health, health } from "../health/index.ts";
import { implementHealth } from "../health/server.ts";
import { createClient } from "../client/index.ts";
import { eventIterator, defineProcedure } from "../service/index.ts";
import { ServiceError } from "../error/index.ts";
import { implement, Server, ServiceHandler } from "./index.ts";

test("drain complete HTTP response streams before disposing service resources", async () => {
    // declare health and a stream whose completion the caller controls
    const release = Promise.withResolvers<void>();
    const readiness = new Health("reader");
    const service = {
        health,
        read: defineProcedure({ authentication: "public", permission: null, audit: false })
            .route({ method: "GET", path: "/read" })
            .output(eventIterator(schema.string())),
    };

    // hold the response open until the caller releases the final event
    const implementation = implement(service);
    const handler = new ServiceHandler<{ caller: string }>(
        implementation.router({
            health: implementHealth(readiness),
            read: implementation.read.handler(async function* () {
                yield "first";
                await release.promise;
                yield "last";
            }),
        }),
        {
            health: readiness,
            authorize: async ({ context }) => {
                if (context.caller !== "alice") {
                    throw new ServiceError("UNAUTHORIZED");
                }
            },
        },
    );

    // retain lifecycle calls while serving the declared procedures
    const lifecycle: string[] = [];
    const server = await Server.start({
        handler,
        context: () => ({ caller: "alice" }),
        initialize: async () => {
            lifecycle.push("initialize");
        },
        dispose: async () => {
            lifecycle.push("dispose");
        },
        drainTimeout: 1000,
    });

    // begin consuming the application stream before shutdown
    const client = createClient(service, {
        url: "https://test.local",
        fetch: (request) => server.fetch(request),
    });
    const stream = await client.read();
    expect(await stream.next()).toEqual({ done: false, value: "first" });

    // observe readiness through a second response stream
    const healthStream = await client.health.watch();
    expect(await healthStream.next()).toEqual({
        done: false,
        value: { name: "reader", status: "serving" },
    });

    // end health subscriptions when draining begins
    const closing = server.close();
    expect(await healthStream.next()).toEqual({
        done: false,
        value: { name: "reader", status: "draining" },
    });
    expect(await healthStream.next()).toEqual({ done: true, value: undefined });

    // refuse new work while retaining resources for the open application stream
    expect(server.close()).toBe(closing);
    expect((await server.fetch(new Request("https://test.local/readyz"))).status).toBe(503);
    expect((await server.fetch(new Request("https://test.local/livez"))).status).toBe(200);
    expect((await server.fetch(new Request("https://test.local/read"))).status).toBe(503);
    expect(lifecycle).toEqual(["initialize"]);

    // finish the response before disposing resources exactly once
    release.resolve();
    expect(await stream.next()).toEqual({ done: false, value: "last" });
    expect(await stream.next()).toEqual({ done: true, value: undefined });
    await closing;
    expect(lifecycle).toEqual(["initialize", "dispose"]);
    expect(server.health.check()).toEqual({ name: "reader", status: "stopped" });
});

test("serialize authentication failures and preserve drain timeout causes", async () => {
    // hold an authenticated request past the drain deadline
    const waiting = Promise.withResolvers<void>();
    const entered = Promise.withResolvers<void>();
    const service = {
        get: defineProcedure({ authentication: "public", permission: null, audit: false })
            .route({ method: "GET", path: "/work" })
            .output(schema.string()),
    };

    // keep the handler active until it can acknowledge cancellation
    const implementation = implement(service);
    const handler = new ServiceHandler(
        implementation.router({
            get: implementation.get.handler(async ({ signal }) => {
                entered.resolve();
                await waiting.promise;
                signal?.throwIfAborted();

                return "complete";
            }),
        }),
        { health: new Health("work") },
    );

    // observe cleanup independently of the caller's shutdown deadline
    let disposed = false;
    const disposal = Promise.withResolvers<void>();
    const server = await Server.start({
        handler,
        context: (request) => {
            if (request.headers.get("authorization") !== "alice") {
                throw new ServiceError("UNAUTHORIZED");
            }

            return {};
        },
        dispose: async () => {
            disposed = true;
            disposal.resolve();
        },
        drainTimeout: 5,
    });

    // reject unauthenticated requests through the typed HTTP client
    const client = createClient(service, {
        url: "https://test.local",
        fetch: (request) => server.fetch(request),
    });
    await expect(client.get()).rejects.toMatchObject({ code: "UNAUTHORIZED", status: 401 });

    // admit an authenticated request and retain its cancellation failure
    const authorized = createClient(service, {
        url: "https://test.local",
        headers: { authorization: "alice" },
        fetch: (request) => server.fetch(request),
    });

    // assert the transport preserves the server's drain timeout cause
    const pending = authorized.get();
    const rejected = expect(pending).rejects.toMatchObject({
        message: "Cannot parse response body, please check the response body and content-type.",
        cause: { name: "TimeoutError", message: "Service drain deadline exceeded." },
    });

    // keep resources alive until overdue work acknowledges cancellation
    await entered.promise;
    try {
        await expect(server.close()).rejects.toMatchObject({ name: "TimeoutError" });
        expect(disposed).toBe(false);
        expect(server.health.status).toBe("draining");
    } finally {
        waiting.resolve();
    }

    // finish cleanup after the request releases its resources
    await rejected;
    await disposal.promise;
    expect(disposed).toBe(true);
    expect(server.health.status).toBe("stopped");
});
