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
