import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";
import { t } from "@destack/locale";

/** How many days a space's logs, spans and metric points stay. */
export const telemetryRetention = defineSetting({
    name: "telemetry.retention",
    title: t`Telemetry retention`,
    description: t`How many days logs, traces and metrics stay.`,
    schema: schema.number().int().min(1).max(3650),
    default: 30,
    scope: "space",
});

/** How many days a space's visits and actions stay. */
export const analyticsRetention = defineSetting({
    name: "analytics.retention",
    title: t`Analytics retention`,
    description: t`How many days visits and actions stay.`,
    schema: schema.number().int().min(1).max(3650),
    default: 395,
    scope: "space",
});
