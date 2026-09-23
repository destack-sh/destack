import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Health, health } from "../health/index.ts";
import { implementHealth } from "../health/server.ts";
import { createClient } from "../client/index.ts";
import { eventIterator, defineProcedure } from "../service/index.ts";
import { implement, Server, type ServerOptions } from "./index.ts";
import { hosting } from "./tests/fixture.ts";
import { Watch } from "../watch/index.ts";
import { ServiceError } from "../error/index.ts";
import type { ServiceContext } from "./context.ts";

test("reauthenticate completed snapshot subscriptions and report revoked access", async () => {
    // expose a finite snapshot subscription through the real service transport
    let requests = 0;
    const service = {
        watch: defineProcedure({ authentication: "identity", permission: null, audit: false })
            .route({ method: "GET", path: "/watch" })
            .output(eventIterator(schema.number())),
    };
    const implementation = implement(service)
        .$context<ServiceContext>()
        .use(({ next }) => next({ context: { application: "snapshot" } }));
    const server = await Server.start({
        ...hosting,
        health: new Health("snapshot"),
        drainTimeout: 1000,
        authenticate: async (request) => {
            requests++;
            if (requests === 3) {
                throw new ServiceError("UNAUTHORIZED");
            }

            return hosting.authenticate(request);
        },
        router: implementation.router({
            watch: implementation.watch.handler(async function* ({ context }) {
                expect(context.requireCaller().authentication.subject.id).toBe("alice");
                expect(context.access().subject?.id).toBe("alice");
                expect(context.application).toBe("snapshot");
                yield requests;
            }),
        }),
    });
    const controller = new AbortController();
    try {
        const client = createClient(service, {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        });
        const stream = Watch.observe(
            (signal) => client.watch(undefined, { signal }),
            controller.signal,
        );

        // renew successful subscriptions without retrying an authorization failure
        expect(await stream.next()).toEqual({ done: false, value: 1 });
        expect(await stream.next()).toEqual({ done: false, value: 2 });
        await expect(stream.next()).rejects.toMatchObject({ code: "UNAUTHORIZED" });
        expect(requests).toBe(3);
    } finally {
        controller.abort();
        await server.close();
    }
});

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
    const router = implementation.router({
        health: implementHealth(readiness),
        read: implementation.read.handler(async function* () {
            yield "first";
            await release.promise;
            yield "last";
        }),
    });

    // retain lifecycle calls while serving the declared procedures
    const lifecycle: string[] = [];
    for (const callback of ["authenticate", "authorizeHost", "authorize"] as const) {
        await expect(
            Server.start({
                ...hosting,
                router,
                health: readiness,
                drainTimeout: 1000,
                [callback]: undefined,
            } as unknown as ServerOptions),
        ).rejects.toThrow(`${callback} must be configured before starting a server`);
    }
    const server = await Server.start({
        ...hosting,
        router,
        health: readiness,
        route: async (request) => {
            const path = new URL(request.url).pathname;
            if (path === "/auth/redirect") {
                return Response.redirect("https://identity.local/sign-in", 303);
            } else if (path === "/auth/keys") {
                return Response.json({ keys: [] });
            } else {
                return undefined;
            }
        },
        responseHeaders: { "Cache-Control": "no-store" },
        initialize: async () => {
            lifecycle.push("initialize");
        },
        dispose: async () => {
            lifecycle.push("dispose");
        },
        drainTimeout: 1000,
    });

    // serve a protocol's public endpoint without invoking application credential verification
    const keys = await server.fetch(new Request("https://test.local/auth/keys"));
    expect([keys.status, keys.headers.get("Cache-Control"), await keys.json()]).toEqual([
        200,
        "no-store",
        { keys: [] },
    ]);

    // retain redirect status and location while applying the deployment's response policy
    const redirect = await server.fetch(new Request("https://test.local/auth/redirect"));
    expect([
        redirect.status,
        redirect.headers.get("Location"),
        redirect.headers.get("Cache-Control"),
    ]).toEqual([303, "https://identity.local/sign-in", "no-store"]);

    // begin consuming the application stream before shutdown
    const client = createClient(service, {
        url: "https://test.local",
        headers: { authorization: "alice" },
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
    expect((await server.fetch(new Request("https://test.local/auth/keys"))).status).toBe(503);
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
    const router = implementation.router({
        get: implementation.get.handler(async ({ signal }) => {
            entered.resolve();
            await waiting.promise;
            signal?.throwIfAborted();

            return "complete";
        }),
    });

    // observe cleanup independently of the caller's shutdown deadline
    let disposed = false;
    const disposal = Promise.withResolvers<void>();
    const server = await Server.start({
        ...hosting,
        router,
        health: new Health("work"),
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

    // release requests that fail construction before authentication or handler dispatch
    const consumed = new Request("https://test.local/work", { method: "POST", body: "used" });
    await consumed.text();
    const failure = await server.fetch(consumed);
    expect(failure.status).toBe(500);
    await failure.arrayBuffer();

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
