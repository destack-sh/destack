import { oc } from "@orpc/contract";
import { schema } from "@destack/schema";
import { PermissionReference, type Permission } from "@destack/access/declare";

/** The access and audit requirements of a procedure. */
export const ProcedureAccess = schema.object({
    /** The required credentials. */
    authentication: schema.enum(["public", "identity", "host"]),
    /** The permission checked on the call's target, null when the handler checks itself. */
    permission: PermissionReference.nullable(),
    /** Whether the call records audit events. */
    audit: schema.boolean(),
});

/** The access and audit requirements of a procedure. */
export type ProcedureAccess = schema.Infer<typeof ProcedureAccess>;

/** Declare a procedure with its access requirements. */
export function defineProcedure(
    access: Omit<ProcedureAccess, "permission"> & { readonly permission: Permission | null },
) {
    return oc.$meta<ProcedureAccess>(access).errors({
        NOT_IMPLEMENTED: { status: 501 },
        UNAUTHORIZED: { status: 401 },
        INSUFFICIENT_AUTHENTICATION: { status: 401 },
        FORBIDDEN: { status: 403 },
        NOT_FOUND: { status: 404 },
        CONFLICT: { status: 409 },
        PRECONDITION_FAILED: { status: 412 },
        MANAGED: { status: 409 },
        MOVED: { status: 421 },
        UNSUPPORTED: { status: 422 },
        RATE_LIMITED: { status: 429 },
        UNAVAILABLE: { status: 503 },
    });
}
