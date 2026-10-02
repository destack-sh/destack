import { Package } from "@destack/package";
import { defineSchema, identifier, schema, canonicalize } from "@destack/schema";
import type { SettingValue } from "../object/setting.ts";
import { SettingReference } from "./setting.ts";
import { SettingPlacement, SettingSelection } from "./placement.ts";

/** A placed value's identity: the row and the revision a resolution read. */
const PlacedValue = SettingPlacement.extend({
    /** The value's identifier. */
    id: identifier("setting"),
    /** The value's revision. */
    revision: schema.number().int().min(1),
});

/** The declaration default, a placed value, or a placed value the declaration no longer accepts. */
export const SettingSource = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The declaration default. */
            kind: schema.literal("default"),
            /** The release declaring the default. */
            package: Package,
        }),
        PlacedValue.extend({
            /** A placed value. */
            kind: schema.literal("value"),
        }),
        PlacedValue.extend({
            /** A placed value the current declaration rejects, skipped by the resolution. */
            kind: schema.literal("invalid"),
        }),
    ]),
);
/** The source of a candidate. */
export type SettingSource = schema.Infer<typeof SettingSource>;

/** An effective value with the sources deciding it. */
export const SettingResolution = Object.assign(
    defineSchema(
        schema.object({
            /** The setting. */
            setting: SettingReference,
            /** The selection the value is resolved for. */
            selection: SettingSelection,
            /** The effective value. */
            value: schema.json(),
            /** The agreeing requirements or the winning sources, then the skipped invalid values. */
            sources: schema.array(SettingSource).min(1),
            /** The other applicable sources in ascending precedence. */
            overridden: schema.array(SettingSource),
            /** Whether a requirement fixes the value. */
            enforcement: schema.enum(["ordinary", "required"]),
        }),
    ),
    {
        /** Read the value a resolution shows at an exact placement, valid or not, or null for none. */
        observed(
            resolution: SettingResolution<unknown>,
            placement: SettingPlacement,
        ): Pick<SettingValue, "id" | "revision"> | null {
            const selected = canonicalize(SettingPlacement.of(placement));
            const source = [...resolution.sources, ...resolution.overridden].find(
                (candidate) =>
                    candidate.kind !== "default" &&
                    canonicalize(SettingPlacement.of(candidate)) === selected,
            );

            return source === undefined || source.kind === "default"
                ? null
                : { id: source.id, revision: source.revision };
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
