import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";

/** Where a value is placed: its scope and the overrides narrowing where it applies. */
const Placement = defineSchema(
    schema.object({
        /** The user, space, account or organisation holding the value. */
        scope: schema.string().min(1),
        /** The consuming package the value applies to. */
        package: PackageId.exactOptional(),
        /** The space the value applies in. */
        space: schema.identifier("space").exactOptional(),
        /** The installation the value applies to. */
        installation: schema.identifier("installation").exactOptional(),
    }),
);

/** Where a value is placed. */
export type SettingPlacement = schema.Infer<typeof Placement>;

/** A placement whose absent overrides may be null. */
type Placed = Pick<SettingPlacement, "scope"> & {
    readonly [Override in Exclude<keyof SettingPlacement, "scope">]?:
        | SettingPlacement[Override]
        | null;
};

/** Where a value is placed: its scope and the overrides narrowing where it applies. */
export const SettingPlacement = Object.assign(Placement, {
    /** Read the placement of a value, source or selection, dropping absent overrides. */
    of: placementOf,
});

/** The scope and overrides a resolution selects. */
export const SettingSelection = defineSchema(
    Placement.extend({
        /** The selected user, space or client, or null for an anonymous visitor. */
        scope: schema.string().min(1).nullable(),
    }),
);
/** The scope and overrides a resolution selects. */
export type SettingSelection = schema.Infer<typeof SettingSelection>;

/** Read a placement, dropping absent overrides. */
function placementOf(placed: Placed): SettingPlacement {
    return {
        scope: placed.scope,
        ...(placed.package == null ? {} : { package: placed.package }),
        ...(placed.space == null ? {} : { space: placed.space }),
        ...(placed.installation == null ? {} : { installation: placed.installation }),
    };
}
