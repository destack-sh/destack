import { none, Policy } from "@destack/access";
import type {} from "@destack/package/import-meta";

/** The logs, spans and metric points of an installation or a scope. */
export const telemetry = new Policy(import.meta.destack.package, {
    name: "telemetry",
    relations: {},
    permissions: { read: none(), unmask: none() },
});

/** The visits and actions of an installation's views. */
export const analytics = new Policy(import.meta.destack.package, {
    name: "analytics",
    relations: {},
    permissions: { read: none(), unmask: none() },
});
