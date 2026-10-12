import { none, Policy } from "@destack/access";
import type {} from "@destack/package/import-meta";

/** The audited calls of a scope, read and shown with their personal values. */
export const audit = new Policy(import.meta.destack.package, {
    name: "audit",
    relations: {},
    permissions: { read: none(), unmask: none() },
});
