import { expect, test } from "@destack/test";
import { SimulatedClock } from "../clock/simulated.ts";
import { SimulatedRandom } from "../random/simulated.ts";
import { SimulatedTelemetry } from "./simulated.ts";

test("count each histogram's records, and database statements apart by their operation and table", async () => {
    // record three selects of one table, an insert into another, a statement of neither and a request
    const telemetry = new SimulatedTelemetry({
        clock: SimulatedClock.owned(0).clock,
        random: new SimulatedRandom(1),
    });
    const meter = telemetry.meters.getMeter("test");
    const statements = meter.createHistogram("db.client.operation.duration");
    const select = { "db.operation.name": "SELECT", "db.collection.name": "note" };
    for (const duration of [1, 2, 3]) {
        statements.record(duration, select);
    }
    statements.record(1, { "db.operation.name": "INSERT", "db.collection.name": "task" });
    statements.record(1, { "db.system.name": "sqlite" });
    meter.createHistogram("rpc.server.call.duration").record(5, { "rpc.method": "get" });

    // count every record of each histogram, and each statement kind beneath the database's count
    expect(await telemetry.counts()).toEqual({
        "db.client.operation.duration.count": 5,
        "db.client.operation.duration.count.SELECT.note": 3,
        "db.client.operation.duration.count.INSERT.task": 1,
        "db.client.operation.duration.count.other.none": 1,
        "rpc.server.call.duration.count": 1,
    });
});
