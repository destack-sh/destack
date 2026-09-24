import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { Creation, defineProcedure } from "@destack/service/procedure";
import { page, PageRequest } from "@destack/service/page";
import { SettingAuthority } from "../setting/target.ts";
import { SettingPolicy } from "../setting/policy.ts";
import { SettingPolicyDefinition } from "../declare/policy.ts";
import type {} from "@destack/package/import-meta";

/** One policy under its administering authority. */
const selection = schema.object({ authority: SettingAuthority, id: SettingPolicy.shape.id });

/** Recommended and required policy administration. */
export const policy = {
    /** Read one policy. */
    get: procedure("get", false)
        .route({ method: "POST", path: "/policy/get" })
        .input(selection)
        .output(SettingPolicy),
    /** List policies administered by one verified authority. */
    list: procedure("list", false)
        .route({ method: "POST", path: "/policy/list" })
        .input(PageRequest.extend({ authority: SettingAuthority }))
        .output(page(SettingPolicy)),
    /** Create or replace a policy at its expected revision. */
    set: procedure("set", true)
        .route({ method: "POST", path: "/policy/set" })
        .input(
            SettingPolicyDefinition.extend(Creation.shape).extend({
                /** Consumer whose selected release validates the policy value. */
                packageId: PackageId,
                /** The stable identity retained for retries and source reconciliation. */
                id: SettingPolicy.shape.id,
                /** Null requires that no policy exists. */
                expectedRevision: schema.uuid().nullable(),
            }),
        )
        .output(SettingPolicy),
    /** Delete a policy at its observed revision. */
    remove: procedure("remove", true)
        .route({ method: "POST", path: "/policy/remove" })
        .input(selection.extend(Creation.shape).extend({ revision: schema.uuid() }))
        .output(schema.null()),
    /** Stop source reconciliation while retaining its last applied provenance. */
    detach: procedure("detach", true)
        .route({ method: "POST", path: "/policy/detach" })
        .input(selection.extend(Creation.shape).extend({ revision: schema.uuid() }))
        .output(SettingPolicy),
};

/** Declare administrative permissions and audited policy mutations. */
function procedure(name: string, audit: boolean) {
    return defineProcedure({
        authentication: "identity",
        permission: { packageId: import.meta.destack.package.id, type: "policy", name },
        audit,
    });
}
