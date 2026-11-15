import { SimulatedTelemetry } from "@destack/environment/simulation";
import { TelemetryScope } from "@destack/telemetry";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { TransactionSpans } from "./spans.ts";

/** The transactions a database keeps the spans of, as `spans.ts` sets it. */
const KEPT_TRANSACTIONS = 4096;

test("keep the active span of each transaction, none outside a span, and forget the oldest beyond the kept ones", async () => {
    // keep the spans of a scope on recorded telemetry
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const telemetry = new SimulatedTelemetry(world);
    const scope = TelemetryScope.of(import.meta.destack.package, telemetry);
    const spans = new TransactionSpans(scope);

    // add one transaction outside a span, then one more than the kept ones inside a span
    spans.add("outside");
    await scope.span("test.write", {}, () => {
        for (let transaction = 0; transaction <= KEPT_TRANSACTIONS; transaction += 1) {
            spans.add(String(transaction));
        }
    });

    // keep the span of the latest transactions alone
    const [written] = telemetry.spans.map((span) => span.spanContext().spanId);
    expect([
        spans.get("outside"),
        spans.get("0"),
        spans.get("1")?.spanId,
        spans.get(String(KEPT_TRANSACTIONS))?.spanId,
    ]).toEqual([undefined, undefined, written, written]);
});
