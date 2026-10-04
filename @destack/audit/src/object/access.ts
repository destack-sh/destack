import { none, Policy, through } from "@destack/access";
import type {} from "@destack/package/import-meta";

/** The audited calls of a scope. */
export const call = new Policy(import.meta.destack.package, {
    name: "call",
    relations: {},
    permissions: { ingest: none(), read: none(), prune: none() },
});

/** An object an audited call names, read with its call. */
export const target = new Policy(import.meta.destack.package, {
    name: "target",
    relations: { call: { subjects: [call] } },
    permissions: { read: through("call", "read") },
});
