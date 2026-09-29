import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Health, health } from "../health/index.ts";
import { implementHealth } from "../health/server.ts";
import { createClient } from "../client/index.ts";
import { eventIterator, defineProcedure } from "../service/index.ts";
import { implement, Server, type ServerOptions } from "./index.ts";
import { createCaller, hosting } from "./tests/fixture.ts";
import { Observable } from "../observable/index.ts";
import { ServiceError } from "../error/index.ts";
import type { ServiceContext } from "./context.ts";
import { Bookmark, Watermark } from "../bookmark/index.ts";
import { Caller } from "../authentication/index.ts";

/** The authentication lifetime of the lapsing callers, short enough for a fast test. */
const LAPSE_MILLISECONDS = 200;

test("reauthenticate completed snapshot subscriptions and report revoked access", async () => {
    // serve a finite snapshot subscription
    let requests = 0;
    const service = {
        watch: defineProcedure({ authentication: "identity", permission: null, audit: false })
            .route({ method: "GET", path: "/watch" })
            .output(eventIterator(schema.number())),
    };
    const implementation = implement(service)
        .$context<ServiceContext>()
        .use(({ next }) => next({ context: { application: "snapshot" } }));
    const server = Server.start({
        ...hosting,
        health: new Health("snapshot"),
        drainTimeout: 1000,
        authenticate: async (request) => {
            requests++;
            if (requests === 3) {
                throw new ServiceError("UNAUTHORIZED", { message: "invalid bearer credential" });
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
        const stream = Observable.observe(
            (signal) => client.watch(undefined, { signal }),
            controller.signal,
        );

        // renew subscriptions but not after an authorization failure
        expect(await stream.next()).toEqual({ done: false, value: 1 });
        expect(await stream.next()).toEqual({ done: false, value: 2 });
        await expect(stream.next()).rejects.toMatchObject({ code: "UNAUTHORIZED" });
        expect(requests).toBe(3);
    } finally {
        controller.abort();
        await server.close();
    }
});

test.for([
    {
        name: "complete the run as verified",
        revoked: false,
        outcome: { values: [1, 2], error: undefined },
    },
    {
        name: "refuse the value once revoked",
        revoked: true,
        outcome: { values: [1], error: "FORBIDDEN" },
    },
] as const)(
    "serve a stream past its caller's lapse to its next consistent point: $name",
    async ({ revoked, outcome }) => {
        // serve a run of two values with the second after the caller's lapse
        let isRevoked = false;
        const service = {
            watch: defineProcedure({ authentication: "identity", permission: null, audit: false })
                .route({ method: "GET", path: "/watch" })
                .output(eventIterator(schema.number())),
        };
        const implementation = implement(service).$context<ServiceContext>();
        const server = Server.start({
            ...hosting,
            health: new Health("lapse"),
            drainTimeout: 1000,
            authenticate: async () => {
                const { authentication } = createCaller("alice");

                return new Caller({
                    ...authentication,
                    expiresAt: Date.now() + LAPSE_MILLISECONDS,
                });
            },
            authorizeHost: async () => {
                if (isRevoked) {
                    throw new ServiceError("FORBIDDEN", { message: "permission denied: watch" });
                }
            },
            router: implementation.router({
                watch: implementation.watch.handler(async function* ({ context }) {
                    yield 1;
                    await new Promise((resolve) =>
                        context.signal.addEventListener("abort", resolve, { once: true }),
                    );
                    isRevoked = revoked;
                    yield 2;
                }),
            }),
        });

        // read every value until the stream ends
        try {
            const client = createClient(service, {
                url: "https://test.local",
                headers: { authorization: "alice" },
                fetch: (request) => server.fetch(request),
            });
            const values: number[] = [];
            const error = await (async () => {
                for await (const value of await client.watch()) {
                    values.push(value);
                }
            })().then(
                () => undefined,
                (failure: ServiceError<string, unknown>) => failure.code,
            );

            // complete the run as verified, unless revoked meanwhile
            expect({ values, error }).toEqual(outcome);
        } finally {
            await server.close();
        }
    },
);

test("drain complete HTTP response streams before reporting the server stopped", async () => {
    // declare health and a held stream
    const release = Promise.withResolvers<void>();
    const readiness = new Health("reader");
    const service = {
        health,
        read: defineProcedure({ authentication: "public", permission: null, audit: false })
            .route({ method: "GET", path: "/read" })
            .output(eventIterator(schema.string())),
    };

    // hold the response until released
    const implementation = implement(service);
    const router = implementation.router({
        health: implementHealth(readiness),
        read: implementation.read.handler(async function* () {
            yield "first";
            await release.promise;
            yield "last";
        }),
    });

    for (const callback of ["authenticate", "authorizeHost"] as const) {
        expect(() =>
            Server.start({
                ...hosting,
                router,
                health: readiness,
                drainTimeout: 1000,
                [callback]: undefined,
            } as unknown as ServerOptions),
        ).toThrow(`${callback} must be configured before starting a server`);
    }
    const server = Server.start({
        ...hosting,
        router,
        health: readiness,
        route: async (request, context) => {
            const path = new URL(request.url).pathname;
            if (path === "/auth/redirect") {
                return Response.redirect("https://identity.local/sign-in", 303);
            } else if (path === "/auth/keys") {
                return Response.json({ keys: [] });
            } else if (path === "/auth/caller") {
                return Response.json({ subject: context.access().subject?.id ?? null });
            } else {
                return undefined;
            }
        },
        responseHeaders: { "Cache-Control": "no-store" },
        drainTimeout: 1000,
    });

    // serve a public protocol endpoint
    const keys = await server.fetch(new Request("https://test.local/auth/keys"));
    expect([keys.status, keys.headers.get("Cache-Control"), await keys.json()]).toEqual([
        200,
        "no-store",
        { keys: [] },
    ]);

    // give the protocol the authenticated caller, and its credential failure
    const caller = async (headers: Record<string, string>) => {
        const response = await server.fetch(
            new Request("https://test.local/auth/caller", { headers }),
        );

        return [response.status, (await response.json()) as unknown];
    };
    expect([await caller({ authorization: "alice" }), (await caller({}))[0]]).toEqual([
        [200, { subject: "alice" }],
        401,
    ]);

    // keep the redirect status and location
    const redirect = await server.fetch(new Request("https://test.local/auth/redirect"));
    expect([
        redirect.status,
        redirect.headers.get("Location"),
        redirect.headers.get("Cache-Control"),
    ]).toEqual([303, "https://identity.local/sign-in", "no-store"]);

    // start a stream before shutdown
    const client = createClient(service, {
        url: "https://test.local",
        headers: { authorization: "alice" },
        fetch: (request) => server.fetch(request),
    });
    const stream = await client.read();
    expect(await stream.next()).toEqual({ done: false, value: "first" });

    // watch health
    const healthStream = await client.health.watch();
    expect(await healthStream.next()).toEqual({
        done: false,
        value: { name: "reader", status: "serving" },
    });

    // end health subscriptions on drain
    const closing = server.close();
    expect(await healthStream.next()).toEqual({
        done: false,
        value: { name: "reader", status: "draining" },
    });
    expect(await healthStream.next()).toEqual({ done: true, value: undefined });

    // refuse new work while the stream stays open
    expect(server.close()).toBe(closing);
    expect((await server.fetch(new Request("https://test.local/readyz"))).status).toBe(503);
    expect((await server.fetch(new Request("https://test.local/livez"))).status).toBe(200);
    expect((await server.fetch(new Request("https://test.local/read"))).status).toBe(503);
    expect((await server.fetch(new Request("https://test.local/auth/keys"))).status).toBe(503);
    expect(server.health.status).toBe("draining");

    // finish the response before stopping
    release.resolve();
    expect(await stream.next()).toEqual({ done: false, value: "last" });
    expect(await stream.next()).toEqual({ done: true, value: undefined });
    await closing;
    expect(server.health.check()).toEqual({ name: "reader", status: "stopped" });
});

test("serialize authentication failures and preserve drain timeout causes", async () => {
    // hold a request past the drain deadline
    const waiting = Promise.withResolvers<void>();
    const entered = Promise.withResolvers<void>();
    const service = {
        get: defineProcedure({ authentication: "public", permission: null, audit: false })
            .route({ method: "GET", path: "/work" })
            .output(schema.string()),
    };

    // hold the handler until cancellation
    const implementation = implement(service);
    const router = implementation.router({
        get: implementation.get.handler(async ({ signal }) => {
            entered.resolve();
            await waiting.promise;
            signal?.throwIfAborted();

            return "complete";
        }),
    });

    // observe stopping
    let isStopped = false;
    const server = Server.start({
        ...hosting,
        router,
        health: new Health("work"),
        drainTimeout: 5,
    });
    void server.stopped.then(() => {
        isStopped = true;
    });

    // reject unauthenticated requests
    const client = createClient(service, {
        url: "https://test.local",
        fetch: (request) => server.fetch(request),
    });
    await expect(client.get()).rejects.toMatchObject({ code: "UNAUTHORIZED", status: 401 });

    // release requests that fail before dispatch
    const consumed = new Request("https://test.local/work", { method: "POST", body: "used" });
    await consumed.text();
    const failure = await server.fetch(consumed);
    expect(failure.status).toBe(500);
    await failure.arrayBuffer();

    // admit an authenticated request
    const authorized = createClient(service, {
        url: "https://test.local",
        headers: { authorization: "alice" },
        fetch: (request) => server.fetch(request),
    });

    // keep the drain timeout cause
    const pending = authorized.get();
    const rejected = expect(pending).rejects.toMatchObject({
        message: "Cannot parse response body, please check the response body and content-type.",
        cause: { name: "TimeoutError", message: "service drain deadline exceeded" },
    });

    // keep resources until cancellation is acknowledged
    await entered.promise;
    try {
        await expect(server.close()).rejects.toMatchObject({ name: "TimeoutError" });
        expect(isStopped).toBe(false);
        expect(server.health.status).toBe("draining");
    } finally {
        waiting.resolve();
    }

    // finish cleanup
    await rejected;
    await server.stopped;
    expect(isStopped).toBe(true);
    expect(server.health.status).toBe("stopped");
});

test("return observed watermarks and require them on the client's later requests", async () => {
    // observe write watermarks and require read watermarks
    const service = {
        write: defineProcedure({ authentication: "identity", permission: null, audit: false })
            .route({ method: "POST", path: "/write" })
            .output(schema.object({})),
        read: defineProcedure({ authentication: "identity", permission: null, audit: false })
            .route({ method: "GET", path: "/read" })
            .output(schema.array(Watermark)),
    };
    const implementation = implement(service).$context<ServiceContext>();
    const server = Server.start({
        ...hosting,
        health: new Health("bookmark"),
        drainTimeout: 1000,
        router: implementation.router({
            write: implementation.write.handler(({ context }) => {
                context.observed.observe({ scope: "space-1", epoch: "epoch-1", sequence: 7 });
                context.observed.observe({ scope: "space-1", epoch: "epoch-1", sequence: 5 });

                return {};
            }),
            read: implementation.read.handler(({ context }) => [...context.bookmark.watermarks]),
        }),
    });
    try {
        const bookmark = new Bookmark();
        const client = createClient(service, {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
            bookmark,
        });
        expect(await client.read()).toEqual([]);
        await client.write();
        expect(bookmark.watermarks).toEqual([{ scope: "space-1", epoch: "epoch-1", sequence: 7 }]);
        expect(await client.read()).toEqual([{ scope: "space-1", epoch: "epoch-1", sequence: 7 }]);
    } finally {
        await server.close();
    }
});

test.for(["before the call", "during the call", "between events"] as const)(
    "end a server stream once its caller aborts %s",
    async (moment) => {
        // count the running generators
        let running = 0;
        const service = {
            follow: defineProcedure({ authentication: "identity", permission: null, audit: false })
                .route({ method: "GET", path: "/follow" })
                .output(eventIterator(schema.number())),
        };
        const implementation = implement(service).$context<ServiceContext>();
        const server = Server.start({
            ...hosting,
            health: new Health("follow"),
            drainTimeout: 1000,
            router: implementation.router({
                follow: implementation.follow.handler(async function* ({ context }) {
                    running += 1;
                    try {
                        yield 1;
                        await new Promise<void>((resolve) => {
                            context.signal.addEventListener("abort", () => resolve(), {
                                once: true,
                            });
                            if (context.signal.aborted) {
                                resolve();
                            }
                        });
                    } finally {
                        running -= 1;
                    }
                }),
            }),
        });

        // abort the call at the moment under test
        const client = createClient(service, {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        });
        const controller = new AbortController();
        if (moment === "before the call") {
            controller.abort();
        }
        const calling = client.follow(undefined, { signal: controller.signal });
        if (moment === "during the call") {
            controller.abort();
        }
        const read = async () => {
            const events = await calling;
            await events.next();
            await collectGarbage();
            if (moment === "between events") {
                controller.abort();
            }

            return await events.next();
        };
        const outcome = await read().then(
            () => "read",
            () => "aborted",
        );

        // end every generator
        const started = performance.now();
        await server.close();
        expect([outcome, running, performance.now() - started < 100]).toEqual(["aborted", 0, true]);
    },
);

/** Collect garbage until weakly held objects are gone. */
async function collectGarbage(): Promise<void> {
    for (let round = 0; round < 3; round++) {
        (globalThis as unknown as { gc: () => void }).gc();
        await new Promise((resolve) => setTimeout(resolve, 10));
    }
}

test("refuse starting a server whose procedures carry payloads the HTTP layer cannot describe", () => {
    // declare a procedure with a custom input check
    const readiness = new Health("reader");
    const service = {
        health,
        read: defineProcedure({ authentication: "public", permission: null, audit: false })
            .route({ method: "GET", path: "/read" })
            .input(schema.object({ name: schema.string().refine((name) => name !== "latest") }))
            .output(schema.string()),
    };
    const implementation = implement(service);
    const router = implementation.router({
        health: implementHealth(readiness),
        read: implementation.read.handler(async () => "read"),
    });

    // refuse it at start instead of answering every request with a failure
    expect(() =>
        Server.start({ ...hosting, router, health: readiness, drainTimeout: 1000 }),
    ).toThrow("unsupported schema check: custom");
});
