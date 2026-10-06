import { defineSchema, schema, toJsonSchema, Version } from "@destack/schema";
import { Expression } from "@destack/db";
import { ModuleMetadata, Package } from "@destack/package";
import { Setting, SettingName } from "../setting/setting.ts";

/** The scope a setting belongs to and the overrides it permits. */
export const SettingScope = defineSchema(
    schema.discriminatedUnion("scope", [
        schema.object({
            /** A value of each user. */
            scope: schema.literal("user"),
            /** The overrides the setting permits. */
            overrides: schema.array(schema.enum(["package", "space", "installation", "client"])),
        }),
        schema.object({
            /** A value of each space. */
            scope: schema.literal("space"),
            /** The overrides the setting permits. */
            overrides: schema.array(schema.literal("installation")),
        }),
        schema.object({
            /** A value of each machine. */
            scope: schema.literal("machine"),
            /** The overrides the setting permits: none. */
            overrides: schema.tuple([]),
        }),
    ]),
);
/** The scope a setting belongs to and the overrides it permits. */
export type SettingScope = schema.Infer<typeof SettingScope>;

/** The serializable fields of a setting declaration. */
export const SettingMetadata = defineSchema(
    schema.object({
        /** The package-local name. */
        name: SettingName,
        /** The label settings views show. */
        title: schema.string().min(1),
        /** The behavior the value controls. */
        description: schema.string().min(1),
        /** The group settings views show it in. */
        group: schema.string().min(1).exactOptional(),
        /** When a consumer applies a changed value. */
        apply: schema.enum(["immediate", "restart"]),
        /** The migration guidance of a deprecated setting. */
        deprecated: schema.string().min(1).exactOptional(),
        /** How placed values combine: the nearest whole value, or each key of a record from its nearest placement. */
        merge: schema.enum(["value", "key"]).exactOptional(),
    }),
);

/** A setting as authored. */
export type SettingDefinition<Value extends schema.Schema = schema.Schema> = schema.Infer<
    typeof SettingMetadata
> &
    SettingScope & {
        /** The value schema. */
        readonly schema: Value;
        /** The value when no placed value applies. */
        readonly default: schema.Infer<Value>;
        /** The value each release computes from a value of an earlier release, read as the column `value`, by the release introducing it. */
        readonly convert?: Readonly<Record<Version, Expression>>;
    };

/** Declare a typed setting. */
export function defineSetting<Value extends schema.Schema>(
    definition: SettingDefinition<Value>,
    module?: ModuleMetadata,
): Setting<Value> {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineSetting").package);

    // validate the serializable fields
    const {
        schema: valueSchema,
        default: defaultValue,
        scope,
        overrides,
        convert,
        ...metadata
    } = definition;
    SettingMetadata.parse(metadata);
    SettingScope.parse({ scope, overrides });

    // require conversions keyed by releases up to the declaring one
    Version.requireUpTo(convert ?? {}, owner.version, definition.name);

    // require a declarative value schema and a valid JSON default
    defineSchema(valueSchema);
    valueSchema.parse(defaultValue);
    schema.json().parse(defaultValue);

    // require a record value for a setting merging its keys
    const described = toJsonSchema(valueSchema);
    if (metadata.merge === "key" && (described["type"] !== "object" || "properties" in described)) {
        throw new TypeError(
            `setting ${definition.name} merges the keys of a value that is no record`,
        );
    }

    return new Setting(owner, definition);
}
