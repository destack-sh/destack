import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { Creation, defineProcedure } from "@destack/service/procedure";
import { PageRequest, page } from "@destack/service/page";
import definition from "../../destack.json" with { type: "json" };
import { SettingReference } from "../setting/setting.ts";
import { SettingTarget } from "../setting/target.ts";
import { SettingAssignment } from "../setting/assignment.ts";
import { SettingEdit } from "../setting/edit.ts";

/** One exact assignment target and declaration. */
const selection = schema.object({ setting: SettingReference, target: SettingTarget });
/** A conditional edit validated by the selected consumer. */
const edit = SettingEdit.extend({
    /** Consumer whose selected release validates the edit. */
    packageId: PackageId,
});

/** Assignment administration. */
export const assignment = {
    /** Read an assignment with exact-target edit permission. */
    get: procedure("get", false)
        .route({ method: "POST", path: "/assignment/get" })
        .input(selection)
        .output(
            schema.object({
                /** The current record, or null when no assignment exists. */
                assignment: SettingAssignment.nullable(),
                /** The result of current permission and provenance checks. */
                edit: schema.enum(["allowed", "denied", "source"]),
            }),
        ),
    /** List assignments at one exact authorized target. */
    list: procedure("list", false)
        .route({ method: "POST", path: "/assignment/list" })
        .input(PageRequest.extend({ target: SettingTarget, packageId: PackageId.optional() }))
        .output(page(SettingAssignment)),
    /** Replace the complete value at an exact target. */
    set: procedure("set", true)
        .route({ method: "POST", path: "/assignment/set" })
        .input(edit.extend({ value: schema.json() }))
        .output(SettingAssignment),
    /** Delete an assignment and expose inherited values. */
    reset: procedure("reset", true)
        .route({ method: "POST", path: "/assignment/reset" })
        .input(edit)
        .output(schema.null()),
    /** Stop source reconciliation while retaining its last applied provenance. */
    detach: procedure("detach", true)
        .route({ method: "POST", path: "/assignment/detach" })
        .input(
            selection
                .extend(Creation.shape)
                .extend({ packageId: PackageId, revision: schema.uuid() }),
        )
        .output(SettingAssignment),
};

/** Declare exact-target assignment permissions and audited mutations. */
function procedure(name: string, audit: boolean) {
    return defineProcedure({
        authentication: "identity",
        permission: { packageId: PackageId.parse(definition.id), type: "assignment", name },
        audit,
    });
}
