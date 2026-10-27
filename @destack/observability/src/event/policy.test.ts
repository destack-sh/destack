import type { EventKind } from "@destack/event";
import { SettingValue } from "@destack/setting/object";
import { ids } from "@destack/stack/test";
import { expect, test } from "@destack/test";
import { log, metric, span, visit } from "../object/index.ts";
import { telemetryRetention } from "../setting/index.ts";
import { TestObservability } from "../test/index.ts";

/** The milliseconds in a day. */
const DAY = 86_400_000;

test("keep a space's telemetry and analytics as long as its settings say, and a machine's as long as the settings' defaults", async () => {
    await using kit = await TestObservability.open();
    const retention = (kind: EventKind, scope: string) =>
        kind.retention(scope, SettingValue.retention(kit.served.database));
    const defaults = {
        log: await retention(log.eventKind(), ids.space),
        visit: await retention(visit.eventKind(), ids.space),
    };

    // keep the space's telemetry for 7 days through its stack
    const key = `${telemetryRetention.package.id}/${telemetryRetention.name}`;
    await kit.apply([], {
        settings: {
            [key]: {
                packageId: telemetryRetention.package.id,
                name: telemetryRetention.name,
                mode: "set",
                value: 7,
            },
        },
    });

    // read the defaults, the setting for spans and metric points, and a machine's log at the setting's default
    expect({
        defaults,
        span: await retention(span.eventKind(), ids.space),
        metric: await retention(metric.eventKind(), ids.space),
        visit: await retention(visit.eventKind(), ids.space),
        machine: await retention(log.eventKind(), kit.space.machine.id),
    }).toEqual({
        defaults: { log: 30 * DAY, visit: 395 * DAY },
        span: 7 * DAY,
        metric: 7 * DAY,
        visit: 395 * DAY,
        machine: 30 * DAY,
    });
});
