import { computeOperation, computeTime, networkTransfer } from "@destack/telemetry/meter";
import { schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { Traffic } from "./traffic.ts";

/** The space the traffic belongs to. */
const SPACE = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000b1");

/** The installation serving the requests. */
const INSTALLATION = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-0000000000b2");

test("read a take's requests, CPU time and transfer each way on the compute and network meters, leaving out zeros", async () => {
    // serve two requests, one sending five bytes in and one streaming three bytes out, and trace the space's own work
    const traffic = new Traffic();
    traffic.countRequest(SPACE, INSTALLATION);
    traffic.countRequest(SPACE, INSTALLATION);
    const sent = new Request("https://space.test/notes", { method: "POST", body: "hello" });
    await traffic.countIngress(SPACE, INSTALLATION, sent).text();
    await traffic.countEgress(SPACE, INSTALLATION, new Response("abc")).text();
    traffic.countTrace(SPACE, { cpuMilliseconds: 4 });

    // read each number on its meter and attributes, by installation or by space
    const by = { scope: SPACE, installation: INSTALLATION };
    expect(Traffic.readings(traffic.take())).toEqual([
        { meter: computeOperation, attributes: {}, ...by, value: 2 },
        { meter: computeTime, attributes: {}, scope: SPACE, value: 4 },
        { meter: networkTransfer, attributes: { direction: "ingress" }, ...by, value: 5 },
        { meter: networkTransfer, attributes: { direction: "egress" }, ...by, value: 3 },
    ]);
});

test("count the process CPU time of an instance's delta metrics export on the compute meter", async () => {
    // report user and system seconds beside another sum, then a cumulative sum and a bare point
    const traffic = new Traffic();
    const sessions = { aggregationTemporality: 1, dataPoints: [point("user", { asInt: "2" })] };
    traffic.countMetrics(
        SPACE,
        INSTALLATION,
        metrics([
            cpuTime(1, [point("user", { asDouble: 0.003 }), point("system", { asDouble: 0.0015 })]),
            { name: "session.count", sum: sessions },
        ]),
    );
    const cumulative = metrics([cpuTime(2, [point("user", { asDouble: 1 })])]);
    const bare = metrics([cpuTime(1, [point("user", {})])]);

    // read the seconds as CPU milliseconds of the installation, and refuse the others
    const count = (exported: typeof bare) =>
        refusal(Promise.resolve().then(() => traffic.countMetrics(SPACE, INSTALLATION, exported)));
    expect([await count(cumulative), await count(bare)]).toEqual([
        ["BAD_REQUEST", "METRIC_UNSUPPORTED", "process.cpu.time must be a delta sum"],
        ["BAD_REQUEST", "POINT_WITHOUT_VALUE", "a point of process.cpu.time has no value"],
    ]);
    expect(Traffic.readings(traffic.take())).toEqual([
        {
            meter: computeTime,
            attributes: {},
            scope: SPACE,
            installation: INSTALLATION,
            value: 4.5,
        },
    ]);
});

/** Write an OTLP/JSON metrics export of one scope's metrics. */
function metrics(items: readonly object[]): object {
    const scoped = { scope: { name: "@example/notes" }, metrics: items };

    return { resourceMetrics: [{ scopeMetrics: [scoped] }] };
}

/** Write a process.cpu.time sum of a temporality with its points. */
function cpuTime(temporality: number, points: readonly object[]): object {
    const sum = { aggregationTemporality: temporality, isMonotonic: true, dataPoints: points };

    return { name: "process.cpu.time", unit: "s", sum };
}

/** Write a number point of a CPU mode at a fixed time. */
function point(mode: string, value: object): object {
    const attributes = [{ key: "cpu.mode", value: { stringValue: mode } }];

    return { attributes, startTimeUnixNano: "1", timeUnixNano: "2", ...value };
}
