import { oc } from "@orpc/contract";
import { DECLARED_ERRORS } from "../error/error.ts";
import { schema, Version } from "@destack/schema";
import { Expression } from "@destack/db";
import { PermissionReference, type Permission, type Policy } from "@destack/access/declare";
import { ObjectTypeReference } from "@destack/sync";

/** Who may call a procedure. */
export const ProcedurePermission = schema.union([
    /** The permission the server checks on the call's target. */
    PermissionReference,
    /** The permission the object store checks on each row the call reads or writes. */
    schema.object({ rows: PermissionReference }),
    /** The permissions the served types declare. */
    schema.literal("rows"),
    /** The kinds of principals that call as themselves. */
    schema.object({ principals: schema.array(ObjectTypeReference).min(1) }),
    /** Anyone, for a public procedure. */
    schema.literal("anyone"),
]);

/** Who may call a procedure. */
export type ProcedurePermission = schema.Infer<typeof ProcedurePermission>;

/** The access and audit requirements of a procedure. */
export const ProcedureAccess = schema.object({
    /** The required credentials. */
    authentication: schema.enum(["public", "identity", "machine"]),
    /** Who may call it. */
    permission: ProcedurePermission,
    /** The category of the event a call records when it ends, or false to record only denials. */
    audit: schema.union([schema.literal(false), schema.enum(["activity", "access"])]),
});

/** The access and audit requirements of a procedure. */
export type ProcedureAccess = schema.Infer<typeof ProcedureAccess>;

/** The declared requirements and conversions of each procedure, parsed once. */
const METAS = new WeakMap<object, ProcedureMeta>();

/** A procedure's declared requirements and the conversions of inputs from earlier releases. */
export const ProcedureMeta = Object.assign(
    ProcedureAccess.extend({
        /** The input fields each release computes from an earlier input's fields. */
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
    meta: Omit<ProcedureMeta, "permission"> & {
        readonly permission:
            | Permission
            | { readonly rows: Permission }
            | "rows"
            | { readonly principals: readonly [Policy, ...Policy[]] }
            | "anyone";
    },
) {
    // refuse anyone on a procedure requiring credentials
    const { permission } = meta;
    if (permission === "anyone" && meta.authentication !== "public") {
        throw new TypeError("a procedure for anyone needs public authentication");
    }

    // keep the principal kinds by reference
    const declared: ProcedurePermission =
        typeof permission === "object" && "principals" in permission
            ? {
                  principals: permission.principals.map((kind) => ({
                      packageId: kind.definition.packageId,
                      type: kind.definition.name,
                  })),
              }
            : permission;

    return oc.$meta<ProcedureMeta>({ ...meta, permission: declared }).errors(DECLARED_ERRORS);
}
