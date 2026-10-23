import { principal } from "@destack/access";
import { PackageId, Package } from "@destack/package";
import { expect, onTestFinished, refusal, test, vi } from "@destack/test";
import { schema } from "@destack/schema";
import { health } from "../health/index.ts";
import { Health } from "./health.ts";
import { implementHealth } from "./health.ts";
import { createClient } from "../client/index.ts";
import { eventIterator, defineProcedure } from "../procedure/index.ts";
import { implement, Server, type ServerOptions } from "./index.ts";
import { authenticateUser, fixturePackage, hosting } from "../test/fixture.ts";
import { Observable } from "../client/observable.ts";
import { CallError } from "../error/index.ts";
import type { RequestContext } from "./context.ts";
import { Bookmark, BOOKMARK_HEADER } from "../procedure/bookmark.ts";
import { Watermark } from "@destack/sync";
import { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";

/** The authentication lifetime of the lapsing callers, which the scenarios' clock moves past. */
const LAPSE_MILLISECONDS = 200;

/** Settle a call to "accepted" or its failure's code, reason and message. */
function settle(call: Promise<unknown>): Promise<string> {
    return refusal(call).then((refused) => (refused === "done" ? "accepted" : refused.join(": ")));
}

test("reauthenticate completed snapshot subscriptions and report revoked access", async () => {
    // serve a finite snapshot subscription
    let requests = 0;
    const procedures = {
        watch: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: { principals: [principal.user, principal.machine, principal.installation] },
            audit: false,
        })
            .route({ method: "GET", path: "/watch" })
            .output(eventIterator(schema.number())),
    };
    const implementation = implement(procedures)
        .$context<RequestContext>()
        .use(({ next }) => next({ context: { application: "snapshot" } }));
    const server = Server.start({
        clock: Clock.system,
        ...hosting,
        health: new Health("snapshot"),
        drainTimeout: 1000,
        authenticate: async (request) => {
            requests++;
            if (requests === 3) {
                throw CallError.http("UNAUTHORIZED", { message: "invalid bearer credential" });
            }

            return hosting.authenticate(request);
        },
        router: implementation.router({
            watch: implementation.watch.handler(async function* ({ context }) {
                expect(context.caller().subject?.id).toBe("alice");
                expect(context.caller().subject?.id).toBe("alice");
                expect(context.application).toBe("snapshot");
                yield requests;
            }),
        }),
    });
    const controller = new AbortController();
    try {
        const client = createClient(
            { router: procedures },
            {
                url: "https://test.local",
                headers: { authorization: "alice" },
                fetch: (request) => server.fetch(request),
            },
        );
        const stream = Observable.observe(
            (signal) => client.watch(undefined, { signal }),
            controller.signal,
        );

        // renew subscriptions but not after an authorization failure
        expect(await stream.next()).toEqual({ done: false, value: 1 });
        expect(await stream.next()).toEqual({ done: false, value: 2 });
        expect(await refusal(stream.next())).toEqual([
            "UNAUTHORIZED",
            "UNAUTHORIZED",
            "invalid bearer credential",
        ]);
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
        const clock = new VirtualClock(Date.now());
        let isRevoked = false;
        const procedures = {
            watch: defineProcedure({
                package: Package.of(fixturePackage),
                authentication: "identity",
                permission: {
                    principals: [principal.user, principal.machine, principal.installation],
                },
                audit: false,
            })
                .route({ method: "GET", path: "/watch" })
                .output(eventIterator(schema.number())),
        };
        const implementation = implement(procedures).$context<RequestContext>();
        const server = Server.start({
            clock,
            ...hosting,
            health: new Health("lapse"),
            drainTimeout: 1000,
            authenticate: async () => {
                const alice = authenticateUser("alice");

                return {
                    ...alice,
                    credential: {
                        ...alice.credential,
                        expiresAt: clock.now() + LAPSE_MILLISECONDS,
                    },
                };
            },
            authorizeMachine: async () => {
                if (isRevoked) {
                    throw CallError.http("FORBIDDEN", { message: "permission denied: watch" });
                }
            },
            router: implementation.router({
                watch: implementation.watch.handler(async function* ({ context }) {
                    yield 1;
                    await new Promise((resolve) => {
                        context.signal.addEventListener("abort", resolve, { once: true });
                    });
                    isRevoked = revoked;
                    yield 2;
                }),
            }),
        });

        // read every value until the stream ends
        try {
            const client = createClient(
                { router: procedures },
                {
                    url: "https://test.local",
                    headers: { authorization: "alice" },
                    fetch: (request) => server.fetch(request),
                },
            );
            const values: number[] = [];
            const error = await (async () => {
                for await (const value of await client.watch()) {
                    // lapse the caller once the first value arrived
                    values.push(value);
                    clock.advance(LAPSE_MILLISECONDS);
                }
            })().then(
                () => undefined,
                (failure: CallError<string, unknown>) => failure.code,
            );

            // complete the run as verified
            expect({ values, error }).toEqual(outcome);
        } finally {
            await server.close();
        }
    },
);

test("drain complete HTTP response streams before reporting the server stopped", async () => {
    // declare health and an open stream
    const release = Promise.withResolvers<void>();
    const readiness = new Health("reader");
    const procedures = {
        health,
        read: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "GET", path: "/read" })
            .output(eventIterator(schema.string())),
    };

    // block the response until released
    const implementation = implement(procedures);
    const router = implementation.router({
        health: implementHealth(readiness),
        read: implementation.read.handler(async function* () {
            yield "first";
            await release.promise;
            yield "last";
        }),
    });

    for (const callback of ["authenticate", "authorizeMachine"] as const) {
        const missing: ServerOptions = {
            clock: Clock.system,
            ...hosting,
            router,
            health: readiness,
            drainTimeout: 1000,
        };
        Reflect.deleteProperty(missing, callback);
        expect(() => Server.start(missing)).toThrow(
            `${callback} must be configured before starting a server`,
        );
    }
    const server = Server.start({
        clock: Clock.system,
        ...hosting,
        router,
        health: readiness,
        fetch: async (request, context) => {
            const path = new URL(request.url).pathname;
            if (path === "/auth/redirect") {
                return Response.redirect("https://identity.local/sign-in", 303);
            } else if (path === "/auth/keys") {
                return Response.json({ keys: [] });
            } else if (path === "/auth/caller") {
                return Response.json({ subject: context.caller().subject?.id ?? null });
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

    // give the protocol the authenticated caller and its credential failure
    const caller = async (headers: Record<string, string>) => {
        const response = await server.fetch(
            new Request("https://test.local/auth/caller", { headers }),
        );

        const body: unknown = await response.json();

        return [response.status, body];
    };
    expect([await caller({ authorization: "alice" }), (await caller({}))[0]]).toEqual([
        [200, { subject: "alice" }],
        401,
    ]);

    // record the redirect status and location
    const redirect = await server.fetch(new Request("https://test.local/auth/redirect"));
    expect([
        redirect.status,
        redirect.headers.get("Location"),
        redirect.headers.get("Cache-Control"),
    ]).toEqual([303, "https://identity.local/sign-in", "no-store"]);

    // start a stream before shutdown
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        },
    );
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

test("end a stream that follows its context signal once the server drains, stopping within the deadline", async () => {
    // declare a stream that yields until its context signal aborts
    const procedures = {
        follow: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "GET", path: "/follow" })
            .output(eventIterator(schema.string())),
    };
    const implementation = implement(procedures).$context<RequestContext>();
    const router = implementation.router({
        follow: implementation.follow.handler(async function* ({ context }) {
            yield "first";
            await new Promise((resolve) => {
                context.signal.addEventListener("abort", resolve, { once: true });
            });
            yield "last";
        }),
    });
    const server = Server.start({
        clock: Clock.system,
        ...hosting,
        router,
        health: new Health("follower"),
        drainTimeout: 5000,
    });

    // open the stream, then drain the server
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        },
    );
    const stream = await client.follow();
    const first = await stream.next();
    const closing = server.close();

    // end the stream with its last value and stop
    expect([first, await stream.next(), await stream.next()]).toEqual([
        { done: false, value: "first" },
        { done: false, value: "last" },
        { done: true, value: undefined },
    ]);
    await closing;
    expect(server.health.status).toBe("stopped");
});

test("end a request's context signal once its response is answered", async () => {
    // record the context of a call that answers at once
    let seen: RequestContext | undefined;
    const procedures = {
        get: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "GET", path: "/answer" })
            .output(schema.string()),
    };
    const implementation = implement(procedures).$context<RequestContext>();
    const router = implementation.router({
        get: implementation.get.handler(({ context }) => {
            seen = context;

            return "answered";
        }),
    });
    const server = Server.start({
        clock: Clock.system,
        ...hosting,
        router,
        health: new Health("answerer"),
        drainTimeout: 5000,
    });
    onTestFinished(() => server.close());

    // call it in process, where the request's signal never ends
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        },
    );
    const answer = await client.get();

    // end the context's signal, and the waits bound to it, with the answer
    expect([answer, seen?.signal.aborted]).toEqual(["answered", true]);
});

test("serialize authentication failures and preserve drain timeout causes", async () => {
    // leave a request open past the drain deadline
    const waiting = Promise.withResolvers<void>();
    const entered = Promise.withResolvers<void>();
    const procedures = {
        get: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "GET", path: "/work" })
            .output(schema.string()),
    };

    // block the handler until cancellation
    const implementation = implement(procedures);
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
        clock: Clock.system,
        ...hosting,
        router,
        health: new Health("work"),
        drainTimeout: 5,
    });
    void server.stopped.then(() => {
        isStopped = true;
    });

    // reject unauthenticated requests
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: (request) => server.fetch(request),
        },
    );
    await expect(client.get()).rejects.toMatchObject({ code: "UNAUTHORIZED", status: 401 });

    // release requests that fail before dispatch
    const consumed = new Request("https://test.local/work", { method: "POST", body: "used" });
    await consumed.text();
    const failure = await server.fetch(consumed);
    expect(failure.status).toBe(500);
    await failure.arrayBuffer();

    // accept an authenticated request
    const authorized = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        },
    );

    // record the drain timeout cause
    const pending = authorized.get();
    const rejected = expect(pending).rejects.toMatchObject({
        message: "Cannot parse response body, please check the response body and content-type.",
        cause: { name: "TimeoutError", message: "server drain deadline exceeded" },
    });

    // retain resources until cancellation is acknowledged
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
    const procedures = {
        write: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: { principals: [principal.user, principal.machine, principal.installation] },
            audit: false,
        })
            .route({ method: "POST", path: "/write" })
            .output(schema.object({})),
        read: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: { principals: [principal.user, principal.machine, principal.installation] },
            audit: false,
        })
            .route({ method: "GET", path: "/read" })
            .output(schema.array(Watermark)),
    };
    const implementation = implement(procedures).$context<RequestContext>();
    const server = Server.start({
        clock: Clock.system,
        ...hosting,
        health: new Health("bookmark"),
        drainTimeout: 1000,
        router: implementation.router({
            write: implementation.write.handler(({ context }) => {
                context.observed.observe({
                    scope: "space-1",
                    log: "log-1",
                    epoch: "epoch-1",
                    sequence: 7,
                });
                context.observed.observe({
                    scope: "space-1",
                    log: "log-1",
                    epoch: "epoch-1",
                    sequence: 5,
                });

                return {};
            }),
            read: implementation.read.handler(({ context }) => [...context.bookmark.watermarks]),
        }),
    });
    try {
        const bookmark = new Bookmark();
        const client = createClient(
            { router: procedures },
            {
                url: "https://test.local",
                headers: { authorization: "alice" },
                fetch: (request) => server.fetch(request),
                bookmark,
            },
        );
        expect(await client.read()).toEqual([]);
        await client.write();
        expect(bookmark.watermarks).toEqual([
            { scope: "space-1", log: "log-1", epoch: "epoch-1", sequence: 7 },
        ]);
        expect(await client.read()).toEqual([
            { scope: "space-1", log: "log-1", epoch: "epoch-1", sequence: 7 },
        ]);
    } finally {
        await server.close();
    }
});

test("require the watermarks a caller's credential has beside its bookmark's", async () => {
    // authenticate callers whose credentials include the position their grant was written at
    const granted = { scope: "universe", log: "log-1", epoch: "epoch-1", sequence: 3 };
    const procedures = {
        read: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: { principals: [principal.user] },
            audit: false,
        })
            .route({ method: "GET", path: "/read" })
            .output(schema.array(Watermark)),
    };
    const implementation = implement(procedures).$context<RequestContext>();
    const server = Server.start({
        clock: Clock.system,
        ...hosting,
        authenticate: async (request: Request) => {
            const authentication = await hosting.authenticate(request);

            return {
                ...authentication,
                credential: { ...authentication.credential, watermarks: [granted] },
            };
        },
        health: new Health("bookmark"),
        drainTimeout: 1000,
        router: implementation.router({
            read: implementation.read.handler(({ context }) => [...context.bookmark.watermarks]),
        }),
    });
    try {
        // read the watermarks a request requires, with and without a separate bookmark
        const read = (header?: string) =>
            createClient(
                { router: procedures },
                {
                    url: "https://test.local",
                    headers: {
                        authorization: "alice",
                        ...(header === undefined ? {} : { [BOOKMARK_HEADER]: header }),
                    },
                    fetch: (request) => server.fetch(request),
                },
            ).read();
        const local = { scope: "space-1", log: "log-1", epoch: "epoch-1", sequence: 7 };
        expect([await read(), await read(JSON.stringify([local]))]).toEqual([
            [granted],
            [local, granted],
        ]);
    } finally {
        await server.close();
    }
});

test.for(["before the call", "during the call", "between events"] as const)(
    "end a server stream once its caller aborts %s",
    async (moment) => {
        // count the running generators
        let running = 0;
        const procedures = {
            follow: defineProcedure({
                package: Package.of(fixturePackage),
                authentication: "identity",
                permission: {
                    principals: [principal.user, principal.machine, principal.installation],
                },
                audit: false,
            })
                .route({ method: "GET", path: "/follow" })
                .output(eventIterator(schema.number())),
        };
        const implementation = implement(procedures).$context<RequestContext>();
        const server = Server.start({
            clock: Clock.system,
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
        const client = createClient(
            { router: procedures },
            {
                url: "https://test.local",
                headers: { authorization: "alice" },
                fetch: (request) => server.fetch(request),
            },
        );
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

/** Collect garbage until weakly referenced objects are gone. */
async function collectGarbage(): Promise<void> {
    // require the runtime's collector
    const collect = globalThis.gc;
    if (collect === undefined) {
        throw new TypeError("collecting garbage needs the runtime's gc");
    }

    // collect a few rounds
    for (let round = 0; round < 3; round++) {
        collect();
        await new Promise((resolve) => {
            setTimeout(resolve, 10);
        });
    }
}

test("report a handler's invalid output as an internal failure and a caller's invalid input as the caller's", async () => {
    // serve a procedure whose handler breaks its output schema
    const routes = {
        echo: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "POST", path: "/echo" })
            .input(schema.object({ text: schema.string().min(3) }))
            .output(schema.object({ text: schema.string().min(3) })),
    };
    const implementation = implement(routes);
    await using server = Server.start({
        clock: Clock.system,
        ...hosting,
        health: new Health("echo"),
        drainTimeout: 1000,
        router: implementation.router({
            echo: implementation.echo.handler(() => ({ text: "no" })),
        }),
    });
    const client = createClient(
        { router: routes },
        {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        },
    );

    expect([
        await settle(client.echo({ text: "hello" })),
        await settle(client.echo({ text: "hi" })),
    ]).toEqual([
        "INTERNAL_SERVER_ERROR: INTERNAL_SERVER_ERROR: internal server error",
        "BAD_REQUEST: INVALID_INPUT: invalid input: text: too small: expected string to have >=3 characters",
    ]);
});

test("delay a request while starting and answer it once ready, and refuse it with a retry pause past the window", async () => {
    // serve a procedure behind a readiness the test opens
    const routes = {
        echo: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "POST", path: "/echo" })
            .input(schema.object({ text: schema.string() }))
            .output(schema.object({ text: schema.string() })),
    };
    const implementation = implement(routes);
    const opened = Promise.withResolvers<void>();
    vi.useFakeTimers();
    onTestFinished(() => {
        vi.useRealTimers();
    });
    await using server = Server.start({
        clock: Clock.system,
        ...hosting,
        health: new Health("echo"),
        drainTimeout: 1000,
        ready: () => opened.promise,
        router: implementation.router({
            echo: implementation.echo.handler(({ input }) => input),
        }),
    });
    const client = createClient(
        { router: routes },
        {
            url: "https://test.local",
            headers: { authorization: "alice" },
            fetch: (request) => server.fetch(request),
        },
    );

    // refuse a request still starting once the window passes
    const refused = server.fetch(new Request("https://test.local/echo", { method: "POST" }));
    await vi.advanceTimersByTimeAsync(1000);
    const starting = await refused;

    // delay a call within the window and answer it once the server opens
    const answered = client.echo({ text: "hello" });
    await vi.advanceTimersByTimeAsync(100);
    opened.resolve();

    expect([
        starting.status,
        starting.headers.get("retry-after"),
        await answered,
        server.health.status,
    ]).toEqual([503, "1", { text: "hello" }, "serving"]);
});

test("refuse waiting for a server stopped while it waits for readiness, ending the wait quietly", async () => {
    // start a server whose readiness waits until its stop aborts it
    const server = Server.start({
        clock: Clock.system,
        ...hosting,
        health: new Health("waiting"),
        drainTimeout: 1000,
        ready: (signal) =>
            new Promise<void>((_resolve, reject) => {
                signal.addEventListener("abort", () => reject(signal.reason), { once: true });
            }),
        router: implement({}).router({}),
    });

    // stop it before it serves
    const serving = refusal(server.serving());
    await server.close();

    expect([await serving, server.health.status]).toEqual([
        ["SERVICE_UNAVAILABLE", "SERVER_STOPPED", "the server stopped before it started serving"],
        "stopped",
    ]);
});

test("refuse a caller authenticated for another package as unauthenticated, auditing the denial", async () => {
    // serve an audited procedure to a caller with a credential for another package
    const procedures = {
        get: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: "access",
        })
            .route({ method: "GET", path: "/work" })
            .output(schema.string()),
    };
    const implementation = implement(procedures);
    const audited: string[] = [];
    const alice = authenticateUser("alice");
    const server = Server.start({
        clock: Clock.system,
        ...hosting,
        router: implementation.router({ get: implementation.get.handler(() => "complete") }),
        health: new Health("work"),
        drainTimeout: 5,
        authenticate: async () => ({
            ...alice,
            credential: {
                ...alice.credential,
                audience: PackageId.parse("package-019f7480-0000-7000-8000-0000000000ff"),
            },
        }),
        audit: async (event) => {
            audited.push(event.context.request.authenticationError?.message ?? "verified");
        },
    });
    onTestFinished(() => server.close());

    // answer unauthorized, recording the failed authentication
    const client = createClient(
        { router: procedures },
        { url: "https://test.local", fetch: (request) => server.fetch(request) },
    );
    expect(await settle(client.get())).toBe(
        "UNAUTHORIZED: AUDIENCE_MISMATCH: caller authentication has a different audience or scope",
    );
    expect(audited).toEqual(["caller authentication has a different audience or scope"]);
});
