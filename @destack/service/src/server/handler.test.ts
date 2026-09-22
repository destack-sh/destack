import { PackageId } from "@destack/package/package";
import packageDefinition from "../../destack.json" with { type: "json" };
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { startTelemetry } from "@destack/telemetry/host";
import { MetricReader } from "@destack/telemetry/metric";
import { SimpleSpanProcessor, type ReadableSpan } from "@destack/telemetry/trace";
import { createClient } from "../client/index.ts";
import { ServiceError } from "../error/index.ts";
import { Health } from "../health/index.ts";
import { defineProcedure, eventIterator } from "../service/index.ts";
import { implement, ServiceHandler, type HandlerOptions } from "./handler.ts";

test("enforce access and audit requirements through streamed HTTP calls", async ({
    onTestFinished,
}) => {
    // collect the telemetry produced by real client and handler calls
    const spans: ReadableSpan[] = [];
    const reader = new CallMetrics();
    const telemetry = await startTelemetry({
        name: "notes",
        version: "1.0.0",
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
    const service = {
        read: defineProcedure({
            authentication: "identity",
            permission: {
                packageId: PackageId.parse(packageDefinition.id),
                type: "notes",
                name: "read",
            },
            audit: true,
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

    // authorize the caller and the complete declared permission
    const health = new Health("notes");
    const authorize: NonNullable<HandlerOptions<{ caller: string }>["authorize"]> = async ({
        context,
        access,
    }) => {
        if (
            context.caller !== "alice" ||
            access.authentication !== "identity" ||
            access.permission?.packageId !== packageDefinition.id ||
            access.permission.type !== "notes" ||
            access.permission.name !== "read"
        ) {
            throw new ServiceError("FORBIDDEN");
        }
    };

    // reject missing enforcement during handler construction
    expect(() => new ServiceHandler(router, { health })).toThrow(
        new TypeError("protected procedures require authorization"),
    );
    expect(() => new ServiceHandler(router, { health, authorize })).toThrow(
        new TypeError("audited procedures require audit recording"),
    );

    // retain audit outcomes through a real HTTP client and handler
    const records: { caller: string; outcome: string }[] = [];
    const handler = new ServiceHandler(router, {
        health,
        authorize,
        audit: async ({ call, outcome }) => {
            records.push({ caller: call.context.caller, outcome });
        },
    });
    const client = createClient(service, {
        url: "https://test.local",
        fetch: async (request) => {
            const response = await handler.handle(request, { context: { caller: "alice" } });

            return response.matched ? response.response : new Response(null, { status: 404 });
        },
    });

    // acknowledge success only after the complete stream finishes
    const values = [];
    for await (const value of await client.read()) {
        values.push(value);
    }
    expect(values).toEqual(["first", "last"]);
    expect(records).toEqual([
        { caller: "alice", outcome: "started" },
        { caller: "alice", outcome: "succeeded" },
    ]);

    // record denied access without entering the application handler
    const denied = createClient(service, {
        url: "https://test.local",
        fetch: async (request) => {
            const response = await handler.handle(request, { context: { caller: "bob" } });

            return response.matched ? response.response : new Response(null, { status: 404 });
        },
    });
    await expect(denied.read()).rejects.toMatchObject({ code: "FORBIDDEN", status: 403 });
    expect(invoked).toBe(1);
    expect(records).toEqual([
        { caller: "alice", outcome: "started" },
        { caller: "alice", outcome: "succeeded" },
        { caller: "bob", outcome: "started" },
        { caller: "bob", outcome: "denied" },
    ]);

    // refuse execution when required audit persistence fails
    const unavailable = new ServiceHandler(router, {
        health,
        authorize,
        audit: async () => {
            throw new ServiceError("UNAVAILABLE", {
                status: 503,
                message: "Audit storage unavailable.",
            });
        },
    });
    const blocked = createClient(service, {
        url: "https://test.local",
        fetch: async (request) => {
            const response = await unavailable.handle(request, { context: { caller: "alice" } });

            return response.matched ? response.response : new Response(null, { status: 404 });
        },
    });
    await expect(blocked.read()).rejects.toMatchObject({
        code: "UNAVAILABLE",
        status: 503,
        message: "Audit storage unavailable.",
    });
    expect(invoked).toBe(1);

    // expose probes independently of application authorization
    const starting = await handler.handle(new Request("https://test.local/readyz"), {
        context: { caller: "" },
    });
    expect(starting.matched && starting.response.status).toBe(503);
    health.set("serving");
    const ready = await handler.handle(new Request("https://test.local/readyz"), {
        context: { caller: "" },
    });
    expect(ready.matched && ready.response.status).toBe(200);

    // retain full call outcomes without request values in metric attributes
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
            attributes: { "rpc.system.name": "orpc", "rpc.method": "read", "error.type": "503" },
            count: 1,
        },
    ];
    expect(calls).toEqual([
        { name: "rpc.client.call.duration", calls: outcomes },
        { name: "rpc.server.call.duration", calls: outcomes },
    ]);
    expect(new Set(spans.map((span) => span.spanContext().traceId)).size).toBe(3);
});

/** Collect metrics directly after the HTTP calls finish. */
class CallMetrics extends MetricReader {
    /** Complete synchronous collection. */
    protected async onForceFlush(): Promise<void> {}
    /** Release the reader. */
    protected async onShutdown(): Promise<void> {}
}
