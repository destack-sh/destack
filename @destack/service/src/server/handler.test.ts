import { none, Policy } from "@destack/access";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { startTelemetry } from "@destack/telemetry/host";
import { MetricReader } from "@destack/telemetry/metric";
import { SimpleSpanProcessor, type ReadableSpan } from "@destack/telemetry/trace";
import { createClient } from "../client/index.ts";
import { defineService } from "../declare/index.ts";
import { conceal, ServiceError } from "../error/index.ts";
import { Health } from "../health/index.ts";
import { defineProcedure, eventIterator } from "../service/index.ts";
import { implement, ServiceHandler, type HandlerOptions } from "./handler.ts";
import type {} from "@destack/package/import-meta";

/** The service the handlers serve, whose release the clients speak. */
const fixture = defineService("fixture", {});

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

    // declare a protected, audited stream
    const notes = new Policy(import.meta.destack.package, {
        name: "notes",
        relations: {},
        permissions: { read: none() },
    });
    const service = {
        read: defineProcedure({
            authentication: "identity",
            permission: notes.permission("read"),
            audit: "access",
        })
            .route({ method: "GET", path: "/notes" })
            .output(eventIterator(schema.string())),
    };
    const implementation = implement(service);
    let invoked = 0;
    const router = implementation.router({
        read: implementation.read.handler(async function* () {
            invoked++;
            yield "first";
            yield "last";
        }),
    });

    // authorize the caller and permission
    const health = new Health("notes");
    const authorize: NonNullable<
        HandlerOptions<{ caller: string; request: Request }>["authorize"]
    > = async ({ context, access }) => {
        // hide the notes from carol, and refuse everyone else but alice
        const denial = new ServiceError("FORBIDDEN", { message: "permission denied: read" });
        if (context.caller === "carol") {
            throw conceal(denial, "no notes");
        } else if (
            context.caller !== "alice" ||
            access.authentication !== "identity" ||
            access.permission?.packageId !== import.meta.destack.package.id ||
            access.permission.type !== "notes" ||
            access.permission.name !== "read"
        ) {
            throw denial;
        }
    };

    // reject missing enforcement
    expect(() => new ServiceHandler(router, { service: fixture, health })).toThrow(
        new TypeError("protected procedures require authorization"),
    );
    expect(() => new ServiceHandler(router, { service: fixture, health, authorize })).toThrow(
        new TypeError("audited procedures require audit recording"),
    );

    // record audit outcomes
    const records: { caller: string; outcome: string }[] = [];
    const handler = new ServiceHandler(router, {
        service: fixture,
        health,
        authorize,
        audit: async ({ call, outcome }) => {
            records.push({ caller: call.context.caller, outcome });
        },
    });
    const client = createClient(defineService("fixture", service), {
        url: "https://test.local",
        fetch: async (request) => {
            const response = await handler.handle(request, {
                context: { caller: "alice", request },
            });

            return response.matched ? response.response : new Response(null, { status: 404 });
        },
    });

    // record success after the stream finishes
    const values = [];
    for await (const value of await client.read()) {
        values.push(value);
    }
    expect(values).toEqual(["first", "last"]);
    expect(records).toEqual([{ caller: "alice", outcome: "success" }]);

    // record a denial without running the handler
    const denied = createClient(defineService("fixture", service), {
        url: "https://test.local",
        fetch: async (request) => {
            const response = await handler.handle(request, { context: { caller: "bob", request } });

            return response.matched ? response.response : new Response(null, { status: 404 });
        },
    });
    await expect(denied.read()).rejects.toMatchObject({ code: "FORBIDDEN", status: 403 });
    expect(invoked).toBe(1);
    expect(records).toEqual([
        { caller: "alice", outcome: "success" },
        { caller: "bob", outcome: "denied" },
    ]);

    // answer a concealed denial as a missing resource, recording the denial
    const bodies: unknown[] = [];
    const concealed = createClient(defineService("fixture", service), {
        url: "https://test.local",
        fetch: async (request) => {
            const response = await handler.handle(request, {
                context: { caller: "carol", request },
            });
            bodies.push(response.matched ? await response.response.clone().json() : undefined);

            return response.matched ? response.response : new Response(null, { status: 404 });
        },
    });
    await expect(concealed.read()).rejects.toMatchObject({ code: "NOT_FOUND", status: 404 });
    expect(bodies).toEqual([
        { defined: true, code: "NOT_FOUND", status: 404, message: "no notes" },
    ]);
    expect(records.at(-1)).toEqual({ caller: "carol", outcome: "denied" });

    // fail the call when its audit fails at the end
    const unavailable = new ServiceHandler(router, {
        service: fixture,
        health,
        authorize,
        audit: async () => {
            throw new ServiceError("UNAVAILABLE", {
                status: 503,
                message: "audit storage unavailable",
            });
        },
    });
    const blocked = createClient(defineService("fixture", service), {
        url: "https://test.local",
        fetch: async (request) => {
            const response = await unavailable.handle(request, {
                context: { caller: "alice", request },
            });

            return response.matched ? response.response : new Response(null, { status: 404 });
        },
    });
    const drained = (async () => {
        for await (const _ of await blocked.read()) {
        }
    })();
    await expect(drained).rejects.toMatchObject({
        code: "UNAVAILABLE",
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
                count: (point.value as { count: number }).count,
            })),
        }))
        .sort((left, right) => left.name.localeCompare(right.name));
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
        attributes: { ...outcome.attributes, "destack.caller.version": fixture.package.version },
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
    const service = {
        watch: defineProcedure({ authentication: "identity", permission: null, audit: "access" })
            .route({ method: "GET", path: "/watch" })
            .output(eventIterator(schema.string())),
    };
    const implementation = implement(service);
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
    const handler = new ServiceHandler(router, {
        service: fixture,
        health: new Health("watch"),
        authorize: async () => {
            if (!isAllowed) {
                throw new ServiceError("FORBIDDEN", { message: "permission denied: read" });
            }
        },
        audit: async ({ outcome }) => {
            outcomes.push(outcome);
        },
    });
    const client = createClient(defineService("fixture", service), {
        url: "https://test.local",
        fetch: async (request) => {
            const result = await handler.handle(request, { context: { request } });

            return result.matched ? result.response : new Response(null, { status: 404 });
        },
    });
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

/** Collect metrics after the calls finish. */
class CallMetrics extends MetricReader {
    /** Complete synchronous collection. */
    protected async onForceFlush(): Promise<void> {}
    /** Release the reader. */
    protected async onShutdown(): Promise<void> {}
}
