import { context, createContextKey, ROOT_CONTEXT, trace } from "@opentelemetry/api";
import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package";
import { OtlpExporter } from "../otlp/index.ts";
import { startTelemetry } from "../host/index.ts";
import { extractContext, injectContext } from "./context.ts";

/** The instrumented package. */
const source = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
    name: "@example/notes",
    version: "2026.9.0",
};

test("carry a span's trace through HTTP headers to a remote parent, ignoring the receiver's active context", async () => {
    const telemetry = await startTelemetry(
        new OtlpExporter(
            async () => {},
            (error) => {
                throw error;
            },
        ).options(source),
    );
    try {
        // inject a span's context into outgoing headers
        const span = telemetry.traces.getTracer(source.name).startSpan("note.sync");
        const headers = new Headers();
        injectContext(headers, trace.setSpan(ROOT_CONTEXT, span), telemetry.propagator);
        span.end();

        // extract it as a remote parent, carrying nothing of the active context
        const inherited = createContextKey("inherited");
        const active = trace.setSpan(ROOT_CONTEXT, span).setValue(inherited, true);
        const extracted = context.with(active, () => extractContext(headers, telemetry.propagator));
        const { traceId, spanId } = span.spanContext();
        expect({
            traceparent: headers.get("traceparent"),
            parent: trace.getSpanContext(extracted),
            inherited: extracted.getValue(inherited),
        }).toEqual({
            traceparent: `00-${traceId}-${spanId}-01`,
            parent: { traceId, spanId, traceFlags: 1, isRemote: true },
            inherited: undefined,
        });
    } finally {
        await telemetry.shutdown();
    }
});
