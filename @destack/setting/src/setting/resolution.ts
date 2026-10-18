import { DeclarationReference } from "@destack/package/declare";
import { Package } from "@destack/package";
import { schema, canonicalize } from "@destack/schema";
import type { SettingValue } from "../object/setting.ts";
import type { DeviceSettingValue } from "../object/device-setting.ts";
import { SettingPlacement, SettingSelection } from "./placement.ts";

/** A placed value's identity: the row and the revision a resolution read. */
const PlacedValue = SettingPlacement.extend({
    /** The value's identifier. */
    id: schema.identifier("setting"),
    /** The value's revision. */
    revision: schema.number().int().min(1),
});

/** A placed value. */
const ValueSource = PlacedValue.extend({
    /** A placed value. */
    kind: schema.literal("value"),
});

/** A value the device stores. */
const DeviceSource = schema.object({
    /** A device's value. */
    kind: schema.literal("device"),
    /** The value's identifier in the device's database. */
    id: schema.identifier("device-setting"),
    /** The value's revision. */
    revision: schema.number().int().min(1),
});

/** The declaration default, a placed or device value, or a value the declaration now rejects. */
export const SettingSource = schema.discriminatedUnion("kind", [
    schema.object({
        /** The declaration default. */
        kind: schema.literal("default"),
        /** The release declaring the default. */
        package: Package,
    }),
    ValueSource,
    DeviceSource,
    schema.object({
        /** A value the current declaration rejects, skipped by the resolution. */
        kind: schema.literal("invalid"),
        /** The skipped value. */
        source: schema.discriminatedUnion("kind", [ValueSource, DeviceSource]),
    }),
]);
/** The source of a candidate. */
export type SettingSource = schema.Infer<typeof SettingSource>;

/** An effective value with the sources deciding it. */
export const SettingResolution = Object.assign(
    schema.object({
        /** The setting. */
        setting: DeclarationReference,
        /** The selection the value is resolved for. */
        selection: SettingSelection,
        /** The effective value. */
        value: schema.json(),
        /** The agreeing requirements or winning sources, then the skipped invalid values. */
        sources: schema.array(SettingSource).min(1),
        /** The other applicable sources in ascending precedence. */
        overridden: schema.array(SettingSource),
        /** Whether a requirement fixes the value, or some key of a merged value. */
        enforcement: schema.enum(["ordinary", "required"]),
        /** How each record key resolved, absent for a setting merging whole values. */
        keys: schema
            .record(
                schema.string(),
                schema.object({
                    /** The source deciding the key. */
                    source: SettingSource,
                    /** The other sources setting the key, in ascending precedence. */
                    overridden: schema.array(SettingSource),
                    /** Whether a requirement fixes the key. */
                    enforcement: schema.enum(["ordinary", "required"]),
                }),
            )
            .exactOptional(),
    }),
    {
        /** Read the value a resolution shows at an exact placement or on the device. */
        observed(
            resolution: SettingResolution<unknown>,
            at: SettingPlacement | "device",
        ): Pick<SettingValue | DeviceSettingValue, "id" | "revision"> | null {
            // list every placed and device source, invalid ones included, and find the one at the placement
            const selected = at === "device" ? at : canonicalize(SettingPlacement.of(at));
            const placed = [...resolution.sources, ...resolution.overridden].flatMap((source) =>
                source.kind === "invalid"
                    ? [source.source]
                    : source.kind === "default"
                      ? []
                      : [source],
            );
            const found = placed.find((source) =>
                source.kind === "device"
                    ? selected === "device"
                    : canonicalize(SettingPlacement.of(source)) === selected,
            );

            return found === undefined ? null : { id: found.id, revision: found.revision };
        },
    },
);
/** An effective value with the sources deciding it. */
export type SettingResolution<Value = schema.Infer<ReturnType<typeof schema.json>>> = Omit<
    schema.Infer<typeof SettingResolution>,
    "value"
> & {
    /** The effective value. */
    value: Value;
};
