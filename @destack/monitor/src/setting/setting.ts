import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

/** How many days a space's entries stay searchable. */
export const telemetryRetention = defineSetting({
    name: "telemetry.retention",
    title: "Telemetry retention",
    description: "How many days logs, traces and metrics stay searchable.",
    schema: schema.number().int().min(1).max(3650),
    default: 30,
    scope: "space",
    overrides: ["installation"],
    apply: "immediate",
});

/** The share of traces a space's workloads keep beside every failed or slow one. */
export const traceSampling = defineSetting({
    name: "telemetry.sampling",
    title: "Trace sampling",
    description: "The share of traces kept, beside every failed or slow one.",
    schema: schema.number().min(0).max(1),
    default: 1,
    scope: "space",
    overrides: ["installation"],
    apply: "restart",
});
