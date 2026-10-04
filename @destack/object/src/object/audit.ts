import * as audit from "@destack/audit/history";
import { field } from "../field/field.ts";
import type { MethodBuilder } from "../method/method.ts";
import { INTRINSIC } from "./intrinsic.ts";
import type { ObjectScope, ObjectType } from "./object.ts";

/** Define a scope type's audited calls as objects. */
export function auditCall<const Scope extends ObjectScope>(scope: Scope) {
    return {
        name: "call",
        plural: "calls",
        [INTRINSIC]: { table: audit.auditCall },
        scope,
        represents: audit.call,
        permissions: audit.call.definition.permissions,
        audited: { reads: true },
        methods: (method: MethodBuilder<typeof audit.auditCall>) => ({
            get: method.get("read"),
            list: method.list("read"),
        }),
    } as const;
}

/** Define the objects a scope type's audited calls name. */
export function auditTarget<const Scope extends ObjectScope>(scope: Scope, call: ObjectType) {
    return {
        name: "target",
        plural: "targets",
        [INTRINSIC]: { table: audit.auditTarget },
        scope,
        represents: audit.target,
        permissions: audit.target.definition.permissions,
        fields: { call: field.reference(call) },
        audited: { reads: true },
        methods: (method: MethodBuilder<typeof audit.auditTarget>) => ({
            list: method.list("read"),
        }),
    } as const;
}
