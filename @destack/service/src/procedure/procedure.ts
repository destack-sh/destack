import { oc } from "@orpc/contract";
import { schema, Version } from "@destack/schema";
import { Expression } from "@destack/db";
import { PermissionReference, type Permission } from "@destack/access/declare";

/** The access and audit requirements of a procedure. */
export const ProcedureAccess = schema.object({
    /** The required credentials. */
    authentication: schema.enum(["public", "identity", "host"]),
    /** The permission checked on the call's target, null when the handler checks itself. */
    permission: PermissionReference.nullable(),
    /** The category of the event a call records when it ends, or false to record only denials. */
    audit: schema.union([schema.literal(false), schema.enum(["activity", "access"])]),
    /** Whether an installation calls it only with its space's grant, which its service binding declares. */
    granted: schema.boolean().exactOptional(),
});

/** The access and audit requirements of a procedure. */
export type ProcedureAccess = schema.Infer<typeof ProcedureAccess>;

/** The declared requirements and conversions of each procedure, parsed once. */
const METAS = new WeakMap<object, ProcedureMeta>();

/** A procedure's declared requirements and the conversions of inputs from earlier releases. */
export const ProcedureMeta = Object.assign(
    ProcedureAccess.extend({
        /** The input fields each release computes from an earlier input's fields, by the release introducing them. */
        convert: schema
            .record(Version, schema.record(schema.string().min(1), Expression.schema))
            .exactOptional(),
    }),
    {
        /** Read a procedure's declared requirements and conversions, parsing them once. */
        of(procedure: { readonly "~orpc": { readonly meta: unknown } }): ProcedureMeta {
            let meta = METAS.get(procedure);
            if (meta === undefined) {
                meta = ProcedureMeta.parse(procedure["~orpc"].meta);
                METAS.set(procedure, meta);
            }

            return meta;
        },
    },
);

/** A procedure's declared requirements and the conversions of inputs from earlier releases. */
export type ProcedureMeta = schema.Infer<typeof ProcedureMeta>;

/** Declare a procedure with its access requirements and the conversions of earlier inputs. */
export function defineProcedure(
    meta: Omit<ProcedureMeta, "permission"> & { readonly permission: Permission | null },
) {
    return oc.$meta<ProcedureMeta>(meta).errors({
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
