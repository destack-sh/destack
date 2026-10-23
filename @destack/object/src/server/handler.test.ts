import { Package } from "@destack/package";
import { AccessError, none, Policy, principal } from "@destack/access";
import { DatabaseError } from "@destack/db";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { startTelemetry } from "@destack/telemetry/bun";
import { MetricReader } from "@destack/telemetry/metric";
import { SimpleSpanProcessor, type ReadableSpan } from "@destack/telemetry/trace";
import { createClient } from "../client/index.ts";
import { conceal, CallError } from "../error/index.ts";
import { Health } from "./health.ts";
import { defineProcedure, eventIterator } from "../procedure/index.ts";
import { fixturePackage } from "../test/fixture.ts";
import { implement, RequestHandler, type HandlerOptions } from "./handler.ts";
import type {} from "@destack/package/import-meta";

/** Hide the notes from carol, and refuse everyone else but alice. */
const authorize: NonNullable<
    HandlerOptions<{ caller: string; request: Request }>["authorize"]
> = async ({ request: state, access }) => {
    // conceal the denial from carol
    const denial = CallError.http("FORBIDDEN", { message: "permission denied: read" });
    const { permission } = access;
    const isNotesRead =
        typeof permission === "object" &&
        "name" in permission &&
        permission.packageId === import.meta.destack.package.id &&
        permission.type === "notes" &&
        permission.name === "read";
    if (state.caller === "carol") {
        throw conceal(denial, "no notes");
    } else if (state.caller !== "alice" || access.authentication !== "identity" || !isNotesRead) {
        throw denial;
    }
};

/** Read the call count of a histogram point. */
function countOf(value: number | { readonly count: number }): number {
    if (typeof value === "number") {
        throw new TypeError("a call duration point is a histogram");
    }

    return value.count;
}

test("enforce access and audit requirements through streamed HTTP calls", async ({
    onTestFinished,
}) => {
    // collect telemetry
    const spans: ReadableSpan[] = [];
    const reader = new CallMetrics();
    const telemetry = await startTelemetry({
        name: "notes",
        version: "2026.9.0",
        report: (error) => {
            throw error;
        },
        traces: {
            spanProcessors: [
                new SimpleSpanProcessor({
                    exporter: {
                        export(batch, complete) {
                            spans.push(...batch);
                            complete({ code: 0 });
                        },
                        async shutdown() {},
                    },
                }),
            ],
        },
        metrics: { readers: [reader] },
        logs: {},
    });
    onTestFinished(() => telemetry.shutdown());

    // declare a protected audited stream
    const notes = new Policy(import.meta.destack.package, {
        name: "notes",
        relations: {},
        permissions: { read: none() },
    });
    const procedures = {
        read: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: notes.permission("read"),
            audit: "access",
        })
            .route({ method: "GET", path: "/notes" })
            .output(eventIterator(schema.string())),
    };
    const implementation = implement(procedures);
    let invoked = 0;
    const router = implementation.router({
        read: implementation.read.handler(async function* () {
            invoked++;
            yield "first";
            yield "last";
        }),
    });

    // report the server's health
    const health = new Health("notes");

    // reject missing enforcement
    expect(() => new RequestHandler(router, { package: fixturePackage, health })).toThrow(
        new TypeError("protected procedures require authorization"),
    );
    expect(
        () => new RequestHandler(router, { package: fixturePackage, health, authorize }),
    ).toThrow(new TypeError("audited procedures require audit recording"));

    // record audit outcomes
    const records: { caller: string; outcome: string }[] = [];
    const handler = new RequestHandler(router, {
        package: fixturePackage,
        health,
        authorize,
        audit: async ({ context, outcome }) => {
            records.push({ caller: context.request.caller, outcome });
        },
    });
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const response = await handler.handle(request, {
                    context: { caller: "alice", request },
                });

                return response.matched ? response.response : new Response(null, { status: 404 });
            },
        },
    );

    // record success after the stream finishes
    const values = [];
    for await (const value of await client.read()) {
        values.push(value);
    }
    expect(values).toEqual(["first", "last"]);
    expect(records).toEqual([{ caller: "alice", outcome: "success" }]);

    // record a denial without running the handler
    const denied = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const response = await handler.handle(request, {
                    context: { caller: "bob", request },
                });

                return response.matched ? response.response : new Response(null, { status: 404 });
            },
        },
    );
    await expect(denied.read()).rejects.toMatchObject({ code: "FORBIDDEN", status: 403 });
    expect(invoked).toBe(1);
    expect(records).toEqual([
        { caller: "alice", outcome: "success" },
        { caller: "bob", outcome: "denied" },
    ]);

    // answer a concealed denial as a missing resource
    const bodies: unknown[] = [];
    const concealed = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const response = await handler.handle(request, {
                    context: { caller: "carol", request },
                });
                bodies.push(response.matched ? await response.response.clone().json() : undefined);

                return response.matched ? response.response : new Response(null, { status: 404 });
            },
        },
    );
    await expect(concealed.read()).rejects.toMatchObject({ code: "NOT_FOUND", status: 404 });
    expect(bodies).toEqual([
        {
            defined: true,
            code: "NOT_FOUND",
            status: 404,
            message: "no notes",
            reason: "NOT_FOUND",
            domain: "http",
        },
    ]);
    expect(records.at(-1)).toEqual({ caller: "carol", outcome: "denied" });

    // fail the call when its audit fails at the end
    const unavailable = new RequestHandler(router, {
        package: fixturePackage,
        health,
        authorize,
        audit: async () => {
            throw CallError.http("SERVICE_UNAVAILABLE", {
                message: "audit storage unavailable",
            });
        },
    });
    const blocked = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const response = await unavailable.handle(request, {
                    context: { caller: "alice", request },
                });

                return response.matched ? response.response : new Response(null, { status: 404 });
            },
        },
    );
    const drained = (async () => {
        await Array.fromAsync(await blocked.read());
    })();
    await expect(drained).rejects.toMatchObject({
        code: "SERVICE_UNAVAILABLE",
        status: 503,
        message: "audit storage unavailable",
    });
    expect(invoked).toBe(2);

    // serve probes without authorization
    const probe = new Request("https://test.local/readyz");
    const starting = await handler.handle(probe, { context: { caller: "", request: probe } });
    expect(starting.matched && starting.response.status).toBe(503);
    health.set("serving");
    const ready = await handler.handle(probe, { context: { caller: "", request: probe } });
    expect(ready.matched && ready.response.status).toBe(200);

    // record call metrics without request values
    await telemetry.flush();
    const collected = await reader.collect();
    const calls = collected.resourceMetrics.scopeMetrics
        .flatMap((scope) => scope.metrics)
        .map((metric) => ({
            name: metric.descriptor.name,
            calls: metric.dataPoints.map((point) => ({
                attributes: point.attributes,
                count: countOf(point.value),
            })),
        }))
        .toSorted((left, right) => left.name.localeCompare(right.name));
    const outcomes = [
        { attributes: { "rpc.system.name": "orpc", "rpc.method": "read" }, count: 1 },
        {
            attributes: { "rpc.system.name": "orpc", "rpc.method": "read", "error.type": "403" },
            count: 1,
        },
        {
            attributes: { "rpc.system.name": "orpc", "rpc.method": "read", "error.type": "404" },
            count: 1,
        },
        {
            attributes: { "rpc.system.name": "orpc", "rpc.method": "read", "error.type": "503" },
            count: 1,
        },
    ];
    const released = outcomes.map((outcome) => ({
        ...outcome,
        attributes: { ...outcome.attributes, "destack.caller.version": fixturePackage.version },
    }));
    expect(calls).toEqual([
        { name: "rpc.client.call.duration", calls: outcomes },
        { name: "rpc.server.call.duration", calls: released },
    ]);
    expect(new Set(spans.map((span) => span.spanContext().traceId)).size).toBe(4);
});

/** Withhold streamed values after access is revoked. */
test("withhold streamed values after access revocation", async () => {
    const waiting = Promise.withResolvers<void>();
    const release = Promise.withResolvers<void>();
    let isAllowed = true;
    let isClosed = false;
    const outcomes: string[] = [];
    const procedures = {
        watch: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "identity",
            permission: { principals: [principal.user, principal.machine, principal.installation] },
            audit: "access",
        })
            .route({ method: "GET", path: "/watch" })
            .output(eventIterator(schema.string())),
    };
    const implementation = implement(procedures);
    const router = implementation.router({
        watch: implementation.watch.handler(async function* () {
            try {
                waiting.resolve();
                await release.promise;
                yield "private";
            } finally {
                isClosed = true;
            }
        }),
    });

    // revoke access during a pending read
    const handler = new RequestHandler(router, {
        package: fixturePackage,
        health: new Health("watch"),
        authorize: async () => {
            if (!isAllowed) {
                throw CallError.http("FORBIDDEN", { message: "permission denied: read" });
            }
        },
        audit: async ({ outcome }) => {
            outcomes.push(outcome);
        },
    });
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const result = await handler.handle(request, { context: { request } });

                return result.matched ? result.response : new Response(null, { status: 404 });
            },
        },
    );
    const received = (async () => {
        const values: string[] = [];
        for await (const value of await client.watch()) {
            values.push(value);
        }

        return values;
    })();
    const rejected = expect(received).rejects.toMatchObject({ code: "FORBIDDEN", status: 403 });
    await waiting.promise;
    isAllowed = false;
    release.resolve();
    await rejected;
    expect(isClosed).toBe(true);
    expect(outcomes).toEqual(["denied"]);
});

test("answer a failure thrown by a handler with the status its code declares", async () => {
    // throw the SERVICE_UNAVAILABLE code from a handler
    const procedures = {
        read: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "GET", path: "/read" })
            .output(schema.string()),
    };
    const implementation = implement(procedures);
    const router = implementation.router({
        read: implementation.read.handler(async () => {
            throw CallError.http("SERVICE_UNAVAILABLE", { message: "the instance is starting" });
        }),
    });
    const handler = new RequestHandler(router, {
        package: fixturePackage,
        health: new Health("read"),
    });

    // answer 503 Service Unavailable with the code and message
    const statuses: number[] = [];
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const result = await handler.handle(request, { context: { request } });
                const response = result.matched
                    ? result.response
                    : new Response(null, { status: 404 });
                statuses.push(response.status);

                return response;
            },
        },
    );
    await expect(client.read()).rejects.toMatchObject({
        code: "SERVICE_UNAVAILABLE",
        status: 503,
        message: "the instance is starting",
    });
    expect(statuses).toEqual([503]);
});

test("answer a lower package's failure with the call error it names, and an unknown failure as internal", async () => {
    // throw a database failure, an access challenge and an unknown failure from handlers
    const procedures = {
        read: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "GET", path: "/read" })
            .input(schema.object({ failure: schema.enum(["duplicate", "challenge", "unknown"]) }))
            .output(schema.string()),
    };
    const failures = {
        duplicate: new DatabaseError("DUPLICATE", "a record with the same unique key exists"),
        challenge: new AccessError("INSUFFICIENT_AUTHENTICATION", "authenticate again", {
            stepUp: { assurance: 2, maxAge: 300_000 },
        }),
        unknown: new Error("the disk is full"),
    };
    const implementation = implement(procedures);
    const router = implementation.router({
        read: implementation.read.handler(async ({ input }) => {
            throw failures[input.failure];
        }),
    });
    const handler = new RequestHandler(router, {
        package: fixturePackage,
        health: new Health("read"),
    });
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const result = await handler.handle(request, { context: { request } });

                return result.matched ? result.response : new Response(null, { status: 404 });
            },
        },
    );

    // receive each failure's mapped code, status, message and details
    const received = await Promise.all(
        (["duplicate", "challenge", "unknown"] as const).map((failure) =>
            client.read({ failure }).then(
                () => "done",
                (error: CallError<string, unknown>) => [
                    error.code,
                    error.status,
                    error.message,
                    error.data,
                ],
            ),
        ),
    );
    expect(received).toEqual([
        ["CONFLICT", 409, "a record with the same unique key exists", undefined],
        ["UNAUTHORIZED", 401, "authenticate again", { assurance: 2, maxAge: 300_000 }],
        ["INTERNAL_SERVER_ERROR", 500, "internal server error", undefined],
    ]);
});

/** Collect metrics after the calls finish. */
class CallMetrics extends MetricReader {
    /** Complete synchronous collection. */
    protected async onForceFlush(): Promise<void> {}
    /** Release the reader. */
    protected async onShutdown(): Promise<void> {}
}
