import { PackageId } from "@destack/package";
import { defineSchema, identifier, schema } from "@destack/schema";

/** Where a value is placed: its scope and the overrides narrowing where it applies. */
export const SettingPlacement = Object.assign(
    defineSchema(
        schema.object({
            /** The user, space, account, organisation or host holding the value. */
            scope: schema.string().min(1),
            /** The consuming package the value applies to. */
            package: PackageId.exactOptional(),
            /** The space the value applies in. */
            space: identifier("space").exactOptional(),
            /** The installation the value applies to. */
            installation: identifier("installation").exactOptional(),
            /** The device the value applies on. */
            device: identifier("device").exactOptional(),
        }),
    ),
    {
        /** Read the placement of a value, source or selection, dropping absent overrides. */
        of(
            placed: Pick<SettingPlacement, "scope"> & {
                readonly [Override in Exclude<keyof SettingPlacement, "scope">]?:
                    | SettingPlacement[Override]
                    | null;
            },
        ): SettingPlacement {
            return {
                scope: placed.scope,
                ...(placed.package == null ? {} : { package: placed.package }),
                ...(placed.space == null ? {} : { space: placed.space }),
                ...(placed.installation == null ? {} : { installation: placed.installation }),
                ...(placed.device == null ? {} : { device: placed.device }),
            };
        },
    },
);
/** Where a value is placed. */
export type SettingPlacement = schema.Infer<typeof SettingPlacement>;

/** The scope and overrides a resolution selects. */
export const SettingSelection = defineSchema(
    SettingPlacement.extend({
        /** The selected user, space or host, or null for an anonymous visitor. */
        scope: schema.string().min(1).nullable(),
    }),
);
/** The scope and overrides a resolution selects. */
export type SettingSelection = schema.Infer<typeof SettingSelection>;
