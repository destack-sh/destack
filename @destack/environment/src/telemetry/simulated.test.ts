import { expect, test } from "@destack/test";
import { SimulatedClock } from "../clock/simulated.ts";
import { SimulatedRandom } from "../random/simulated.ts";
import { SimulatedTelemetry } from "./simulated.ts";

test("count each histogram's records and seconds, in all and by the namespace, operation, table and method they name", async () => {
    // record three selects of one table, an insert into another, a statement of neither and a request
    const telemetry = new SimulatedTelemetry({
        clock: SimulatedClock.owned(0).clock,
        random: new SimulatedRandom(1),
    });
    const meter = telemetry.meters.getMeter("test");
    const statements = meter.createHistogram("db.client.operation.duration", { unit: "s" });
    const select = {
        "db.namespace": "main.db",
        "db.operation.name": "SELECT",
        "db.collection.name": "note",
    };
    for (const duration of [1, 2, 3]) {
        statements.record(duration, select);
    }
    statements.record(1, {
        "db.namespace": "memory",
        "db.operation.name": "INSERT",
        "db.collection.name": "task",
    });
    statements.record(1, { "db.system.name": "sqlite" });
    meter
        .createHistogram("rpc.server.call.duration", { unit: "s" })
        .record(5, { "rpc.method": "get" });

    // count and add up every histogram, and each target apart, a statement naming none only in all
    expect(await telemetry.counts()).toEqual({
        "db.client.operation.duration.count": 5,
        "db.client.operation.duration.seconds": 8,
        "db.client.operation.duration.count.main.db:SELECT:note": 3,
        "db.client.operation.duration.seconds.main.db:SELECT:note": 6,
        "db.client.operation.duration.count.memory:INSERT:task": 1,
        "db.client.operation.duration.seconds.memory:INSERT:task": 1,
        "rpc.server.call.duration.count": 1,
        "rpc.server.call.duration.seconds": 5,
        "rpc.server.call.duration.count.get": 1,
        "rpc.server.call.duration.seconds.get": 5,
    });
});

test("name each host on its signals", async () => {
    // emit a span, a record and a metric from two hosts and from the world itself
    const telemetry = new SimulatedTelemetry({
        clock: SimulatedClock.owned(0).clock,
        random: new SimulatedRandom(1),
    });
    const emitters = [telemetry.host("home"), telemetry.host("laptop"), telemetry];
    for (const [index, emitter] of emitters.entries()) {
        emitter.tracers.getTracer("test").startSpan(`span ${index}`).end();
        emitter.loggers.getLogger("test").emit({ body: `record ${index}` });
        emitter.meters
            .getMeter("test")
            .createCounter("count")
            .add(index + 1);
    }

    // read every signal back with the host it names, the world's naming none, and reuse each host's telemetry
    const metrics = await telemetry.metrics();
    expect({
        spans: telemetry.spans.map((span) => [span.name, hostOf(span.resource.attributes)]),
        logs: telemetry.logs.map((record) => [record.body, hostOf(record.resource.attributes)]),
        metrics: metrics.flatMap((each) =>
            each.scopeMetrics.map((scope) => [
                scope.metrics[0]?.dataPoints[0]?.value,
                hostOf(each.resource.attributes),
            ]),
        ),
        isReused: telemetry.host("home") === emitters[0],
    }).toEqual({
        spans: [
            ["span 0", "home"],
            ["span 1", "laptop"],
            ["span 2", null],
        ],
        logs: [
            ["record 0", "home"],
            ["record 1", "laptop"],
            ["record 2", null],
        ],
        metrics: [
            [3, null],
            [1, "home"],
            [2, "laptop"],
        ],
        isReused: true,
    });
});

/** Read the host a signal's resource names, null for the world's own. */
function hostOf(attributes: Record<string, unknown>): unknown {
    return attributes["host.name"] ?? null;
}
