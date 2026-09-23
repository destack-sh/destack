import { PackageId } from "@destack/package";
import { identifier, schema } from "@destack/schema";
import { eventIterator } from "@destack/service";
import { defineProcedure } from "@destack/service/procedure";
import { page, PageRequest } from "@destack/service/page";
import definition from "../../destack.json" with { type: "json" };
import { SettingReference } from "../setting/setting.ts";
import { SettingSelection } from "../setting/target.ts";
import { SettingDescription } from "../inspect/setting.ts";
import { SettingResolution } from "../setting/resolution.ts";

/** Maximum number of declarations resolved in one request or subscription. */
export const SETTING_BATCH_LIMIT = 128;

/** A value request checked against the caller and the receiving installation. */
export const SettingQuery = schema.object({
    /** The consumer whose actual release is selected and verified by the host. */
    packageId: PackageId,
    /** Declarations selected from the release or its resolved dependencies. */
    settings: schema.array(SettingReference).min(1).max(SETTING_BATCH_LIMIT),
    /** The requested target, verified against host context before use. */
    target: SettingSelection,
});

/** Declaration discovery and authorized value resolution. */
export const setting = {
    /** List declarations visible through a specific consumer release. */
    list: procedure("list")
        .route({ method: "POST", path: "/setting/list" })
        .input(
            PageRequest.extend({
                packageId: PackageId,
                installationId: identifier("installation").optional(),
            }),
        )
        .output(page(SettingDescription)),
    /** Resolve unique declarations in first-requested order under one database snapshot. */
    resolve: procedure("resolve")
        .route({ method: "POST", path: "/setting/resolve" })
        .input(SettingQuery)
        .output(schema.array(SettingResolution)),
    /** Yield complete current batches until cancellation or access expiry. */
    watch: procedure("watch")
        .route({ method: "POST", path: "/setting/watch" })
        .input(SettingQuery)
        .output(eventIterator(schema.array(SettingResolution))),
};

/** Declare setting read permissions; hosts verify the consumer and requested target. */
function procedure(name: string) {
    return defineProcedure({
        authentication: "identity",
        permission: { packageId: PackageId.parse(definition.id), type: "setting", name },
        audit: false,
    });
}
