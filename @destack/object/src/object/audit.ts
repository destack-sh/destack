import * as audit from "@destack/audit/history";
import { field } from "../field/field.ts";
import { method } from "../method/method.ts";
import { INTRINSIC } from "./intrinsic.ts";
import type { ObjectScope, ObjectType } from "./object.ts";

/** Define a scope type's audit events as objects. */
export function auditEvent<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "event",
        plural: "events",
        [INTRINSIC]: { table: audit.auditEvent },
        scope,
        represents: audit.event,
        permissions: audit.event.definition.permissions,
        audited: { reads: true },
        methods: { get: method.get("read"), list: method.list("read") },
    } as const;
}

/** Define the objects a scope type's audit events name. */
export function auditTarget<const Scope extends ObjectScope>(scope: Scope, event: ObjectType) {
    return {
        name: "target",
        plural: "targets",
        [INTRINSIC]: { table: audit.auditTarget },
        scope,
        represents: audit.target,
        permissions: audit.target.definition.permissions,
        fields: { event: field.reference(event) },
        audited: { reads: true },
        methods: { list: method.list("read") },
    } as const;
}
