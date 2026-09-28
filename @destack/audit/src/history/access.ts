import { none, Policy, through } from "@destack/access";
import type {} from "@destack/package/import-meta";

/** The audit events of a scope. */
export const event = new Policy(import.meta.destack.package, {
    name: "event",
    relations: {},
    permissions: { ingest: none(), read: none(), prune: none() },
});

/** An object an audit event names, read with its event. */
export const target = new Policy(import.meta.destack.package, {
    name: "target",
    relations: { event: { subjects: [event] } },
    permissions: { read: through("event", "read") },
});
