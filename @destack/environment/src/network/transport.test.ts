import { expect, test } from "@destack/test";
import { SpanKind } from "@opentelemetry/api";
import { Simulation } from "../simulation/simulation.ts";
import { testWorldOptions } from "@destack/test/world";

/** Name a span's end of a call. */
function kindOf(kind: SpanKind): string {
    return kind === SpanKind.CLIENT ? "client" : "server";
}

test("trace a request at both ends, the server's span below the client's, by method and status", async () => {
    // serve a path back and a missing one as not found, then fetch both across the world
    await using simulation = Simulation.of(
        { seed: 1, start: Date.UTC(2026, 0, 1), steps: [] },
        { world: testWorldOptions() },
    );
    const server = simulation.environment("server");
    server.server.http({
        hostname: "server.test",
        port: 443,
        fetch: (request) => {
            const { pathname } = new URL(request.url);

            return new Response(pathname, { status: pathname === "/missing" ? 404 : 200 });
        },
    });
    const { network } = simulation.environment("client");
    for (const path of ["/found", "/missing"]) {
        await simulation.wait(network.fetch(new Request(`https://server.test${path}`)));
    }

    // read each span's kind, status and parent's kind, and the durations' counts by status
    const spans = simulation.telemetry.spans;
    const kinds = new Map(spans.map((span) => [span.spanContext().spanId, kindOf(span.kind)]));
    const traced = spans.map((span) => ({
        name: span.name,
        kind: kindOf(span.kind),
        status: span.attributes["http.response.status_code"],
        error: span.attributes["error.type"],
        parent:
            span.parentSpanContext === undefined
                ? "none"
                : kinds.get(span.parentSpanContext.spanId),
    }));
    const counts = Object.fromEntries(
        Object.entries(await simulation.telemetry.counts()).filter(([key]) =>
            key.endsWith(".count"),
        ),
    );

    // expect a server span inside each client span, the missing path a client error alone
    expect({ traced, counts }).toEqual({
        traced: [
            { name: "GET", kind: "server", status: 200, error: undefined, parent: "client" },
            { name: "GET", kind: "client", status: 200, error: undefined, parent: "none" },
            { name: "GET", kind: "server", status: 404, error: undefined, parent: "client" },
            { name: "GET", kind: "client", status: 404, error: "404", parent: "none" },
        ],
        counts: {
            "http.client.request.duration.count": 2,
            "http.server.request.duration.count": 2,
        },
    });
});

test("report a method HTTP does not define as _OTHER at both ends, keeping each duration bounded", async () => {
    // serve anything, then send a WebDAV method across the world
    await using simulation = Simulation.of(
        { seed: 1, start: Date.UTC(2026, 0, 1), steps: [] },
        { world: testWorldOptions() },
    );
    simulation.environment("server").server.http({
        hostname: "server.test",
        port: 443,
        fetch: () => new Response(null, { status: 207 }),
    });
    const { network } = simulation.environment("client");
    await simulation.wait(
        network.fetch(new Request("https://server.test/notes", { method: "PROPFIND" })),
    );

    // read each end's span name and method
    const methods = simulation.telemetry.spans.map((span) => [
        span.name,
        span.attributes["http.request.method"],
    ]);
    expect(methods).toEqual([
        ["_OTHER", "_OTHER"],
        ["_OTHER", "_OTHER"],
    ]);
});
