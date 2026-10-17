import { setting } from "@destack/setting/object";
import { expect, test } from "@destack/test";
import { log, metric, span, visit } from "../src/object/index.ts";
import { telemetryRetention } from "../src/setting/index.ts";
import { TelemetryPolicy } from "../src/index.ts";
import { ids, openSpace, START } from "./fixture/space.ts";

/** The milliseconds in a day. */
const DAY = 86_400_000;

test("keep a space's telemetry and analytics as long as its settings say, and leave a machine's and other kinds to their own", async () => {
    const space = await openSpace();
    const policy = TelemetryPolicy.of(space.store.database);
    const retention = async (kind: Parameters<typeof policy>[0], scope: string) =>
        (await policy(kind, scope)).retention;
    const defaults = {
        log: await retention(log.eventKind(), ids.space),
        visit: await retention(visit.eventKind(), ids.space),
    };

    // keep the space's telemetry for 7 days
    await space.store.database.insert(setting.table).values({
        id: setting.generateId(),
        scope: ids.space,
        packageId: telemetryRetention.package.id,
        name: telemetryRetention.name,
        mode: "set",
        value: 7,
        release: "2026.9.0",
        createdAt: START,
        updatedAt: START,
    });

    // read the defaults, the setting for spans and metric points, and a machine's log at its kind's own retention
    expect({
        defaults,
        span: await retention(span.eventKind(), ids.space),
        metric: await retention(metric.eventKind(), ids.space),
        visit: await retention(visit.eventKind(), ids.space),
        machine: await retention(log.eventKind(), ids.machine),
    }).toEqual({
        defaults: { log: 30 * DAY, visit: 395 * DAY },
        span: 7 * DAY,
        metric: 7 * DAY,
        visit: 395 * DAY,
        machine: 30 * DAY,
    });
});
