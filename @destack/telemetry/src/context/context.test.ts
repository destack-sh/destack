import { createContextKey, ROOT_CONTEXT, SpanKind, trace } from "@opentelemetry/api";
import { Simulation } from "@destack/environment/simulation";
import { expect, test } from "@destack/test";
import { extractContext, injectContext } from "./context.ts";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

test("join a client span and its server span in one trace across inject and extract on a world", async () => {
    // start a client span on the world's telemetry and inject its context into outgoing headers
    await using simulation = new Simulation({ seed: 1, start: START });
    const { telemetry } = simulation.environment("client");
    const tracer = telemetry.tracers.getTracer("@example/notes");
    const client = tracer.startSpan("note.sync", { kind: SpanKind.CLIENT });
    const headers = new Headers();
    injectContext(headers, telemetry, trace.setSpan(telemetry.context.active(), client));

    // extract it on the server side, carrying nothing of the server's active context
    const inherited = createContextKey("inherited");
    const active = ROOT_CONTEXT.setValue(inherited, true);
    const extracted = telemetry.context.with(active, () => extractContext(headers, telemetry));
    const server = tracer.startSpan("note.receive", { kind: SpanKind.SERVER }, extracted);
    server.end();
    client.end();

    // read both spans in one trace, the server's a child of the client's
    const { traceId, spanId } = client.spanContext();
    const spans = simulation.telemetry.spans.map((span) => ({
        name: span.name,
        traceId: span.spanContext().traceId,
        parent: span.parentSpanContext?.spanId,
    }));
    expect({ spans, inherited: extracted.getValue(inherited) }).toEqual({
        spans: [
            { name: "note.receive", traceId, parent: spanId },
            { name: "note.sync", traceId, parent: undefined },
        ],
        inherited: undefined,
    });
});
