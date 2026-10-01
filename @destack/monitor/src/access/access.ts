import { none, Policy } from "@destack/access";
import { defineAuditAction } from "@destack/audit/declare";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** The prefix of attribute keys with masked values for readers without unmask permission. */
export const SENSITIVE_PREFIX = "sensitive.";

/** The entries of an installation, read through permissions roles grant on it or its space. */
export const entry = new Policy(import.meta.destack.package, {
    name: "entry",
    relations: {},
    permissions: {
        "read-logs": none(),
        "tail-logs": none(),
        "read-traces": none(),
        "read-metrics": none(),
        unmask: none(),
    },
});

/** A read of an installation's entries showing their sensitive values unmasked. */
export const monitorUnmask = defineAuditAction({
    name: "monitor.unmask",
    targets: schema.object({
        /** The installation, or the scope's own object such as a host, whose entries were unmasked. */
        emitter: schema.object({ type: schema.string().min(1), id: schema.string().min(1) }),
    }),
    details: schema.object({ operation: schema.enum(["search", "tail", "trace"]) }),
});
