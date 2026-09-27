import { oc } from "@orpc/contract";
import { schema } from "@destack/schema";
import { PermissionReference, type Permission } from "@destack/access/declare";

/** Access and audit requirements interpreted by the service's middleware. */
export const ProcedureAccess = schema.object({
    /** Credentials required before invoking the procedure. */
    authentication: schema.enum(["public", "identity", "host"]),
    /** The permission the service checks on the call's target before the handler, null when the handler decides through the authorizer itself. */
    permission: PermissionReference.nullable(),
    /** Whether successful and failed attempts require security audit records. */
    audit: schema.boolean(),
});

/** Access and audit requirements declared by a procedure. */
export type ProcedureAccess = schema.Infer<typeof ProcedureAccess>;

/** Declare a procedure with explicit access requirements, a permission some policy declares, and conventional errors. */
export function defineProcedure(
    access: Omit<ProcedureAccess, "permission"> & { readonly permission: Permission | null },
) {
    return oc.$meta<ProcedureAccess>(access).errors({
        NOT_IMPLEMENTED: { status: 501 },
        UNAUTHORIZED: { status: 401 },
        FORBIDDEN: { status: 403 },
        NOT_FOUND: { status: 404 },
        CONFLICT: { status: 409 },
        PRECONDITION_FAILED: { status: 412 },
        MANAGED: { status: 409 },
        UNSUPPORTED: { status: 422 },
        RATE_LIMITED: { status: 429 },
        UNAVAILABLE: { status: 503 },
    });
}
