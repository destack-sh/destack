import { oc } from "@orpc/contract";
import { schema } from "@destack/schema";

/** Access and audit requirements interpreted by the service's middleware. */
export const ProcedureAccess = schema.object({
    /** Credentials required before invoking the procedure. */
    authentication: schema.enum(["public", "identity", "host"]),
    /** Resource type and action checked in the request's authorized scope. */
    permission: schema
        .object({ resource: schema.string().min(1), action: schema.string().min(1) })
        .nullable(),
    /** Whether successful and failed attempts require security audit records. */
    audit: schema.boolean(),
});

/** Access and audit requirements declared by a procedure. */
export type ProcedureAccess = schema.Infer<typeof ProcedureAccess>;

/** Declare a procedure with explicit access requirements and conventional errors. */
export function defineProcedure(access: ProcedureAccess) {
    return oc.$meta<ProcedureAccess>(access).errors({
        NOT_IMPLEMENTED: { status: 501 },
        UNAUTHORIZED: { status: 401 },
        FORBIDDEN: { status: 403 },
        NOT_FOUND: { status: 404 },
        CONFLICT: { status: 409 },
        PRECONDITION_FAILED: { status: 412 },
        SOURCE_MANAGED: { status: 409 },
        UNSUPPORTED: { status: 422 },
        RATE_LIMITED: { status: 429 },
        UNAVAILABLE: { status: 503 },
    });
}
