import type { EventAccess } from "@destack/event/declare";
import { schema } from "@destack/schema";
import { installation } from "@destack/space/object";
import { analytics, telemetry } from "../access/index.ts";

/** The installation a read of a scope's telemetry or analytics narrows to, whose own grants then decide. */
const emitter = {
    key: "installation",
    reference: (scope: string, id: string) =>
        installation.reference(scope, schema.identifier("installation").parse(id)),
};

/** Who reads logs, spans and metric points: telemetry readers of the scope or the installation. */
export const TELEMETRY_ACCESS: EventAccess = {
    read: telemetry.permission("read"),
    unmask: telemetry.permission("unmask"),
    object: emitter,
};

/** Who reads visits and actions: analytics readers of the scope or the installation. */
export const ANALYTICS_ACCESS: EventAccess = {
    read: analytics.permission("read"),
    unmask: analytics.permission("unmask"),
    object: emitter,
};
