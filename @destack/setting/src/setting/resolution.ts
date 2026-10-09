import { Package } from "@destack/package";
import { defineSchema, schema, canonicalize } from "@destack/schema";
import type { SettingValue } from "../object/setting.ts";
import type { ClientSettingValue } from "../object/client.ts";
import { SettingReference } from "./setting.ts";
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

/** A value the client keeps. */
const ClientSource = schema.object({
    /** A client's value. */
    kind: schema.literal("client"),
    /** The value's identifier in the client's database. */
    id: schema.identifier("client-setting"),
    /** The value's revision. */
    revision: schema.number().int().min(1),
});

/** The declaration default, a placed or client value, or a value the declaration no longer accepts. */
export const SettingSource = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The declaration default. */
            kind: schema.literal("default"),
            /** The release declaring the default. */
            package: Package,
        }),
        ValueSource,
        ClientSource,
        schema.object({
            /** A value the current declaration rejects, skipped by the resolution. */
            kind: schema.literal("invalid"),
            /** The skipped value. */
            source: schema.discriminatedUnion("kind", [ValueSource, ClientSource]),
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
            /** The agreeing requirements or the winning sources, followed by the skipped invalid values. */
            sources: schema.array(SettingSource).min(1),
            /** The other applicable sources in ascending precedence. */
            overridden: schema.array(SettingSource),
            /** Whether a requirement fixes the value, or some key of a merged value. */
            enforcement: schema.enum(["ordinary", "required"]),
            /** How each key of a merged value resolved, absent for a setting merging whole values. */
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
    ),
    {
        /** Read the value a resolution shows at an exact placement or on the client, valid or not, or null for none. */
        observed(
            resolution: SettingResolution<unknown>,
            at: SettingPlacement | "client",
        ): Pick<SettingValue | ClientSettingValue, "id" | "revision"> | null {
            // list every placed and client source, invalid ones included, and find the one at the placement
            const selected = at === "client" ? at : canonicalize(SettingPlacement.of(at));
            const placed = [...resolution.sources, ...resolution.overridden].flatMap((source) =>
                source.kind === "invalid"
                    ? [source.source]
                    : source.kind === "default"
                      ? []
                      : [source],
            );
            const found = placed.find((source) =>
                source.kind === "client"
                    ? selected === "client"
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
