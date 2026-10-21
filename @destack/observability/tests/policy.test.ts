import { setting, SettingValue } from "@destack/setting/object";
import { expect, test } from "@destack/test";
import { log, metric, span, visit } from "../src/object/index.ts";
import { telemetryRetention } from "../src/setting/index.ts";
import type { EventKind } from "@destack/event";
import { ids, openSpace, START } from "./fixture/space.ts";

/** The milliseconds in a day. */
const DAY = 86_400_000;

test("keep a space's telemetry and analytics as long as its settings say, and a machine's as long as the settings' defaults", async () => {
    const space = await openSpace();
    const retention = (kind: EventKind, scope: string) =>
        kind.retention(scope, SettingValue.retention(space.store.database));
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

    // read the defaults, the setting for spans and metric points, and a machine's log at the setting's default
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
