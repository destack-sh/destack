import { schema, toJsonSchema, Version } from "@destack/schema";
import { Expression } from "@destack/db";
import { ModuleMetadata, Package } from "@destack/package";
import { Setting, SettingMetadata, SettingScope } from "../setting/setting.ts";

/** A setting as authored. */
export type SettingDefinition<Value extends schema.Schema = schema.Schema> = schema.Infer<
    typeof SettingMetadata
> &
    SettingScope & {
        /** The value schema. */
        readonly schema: Value;
        /** The value when no placed value applies. */
        readonly default: schema.Infer<Value>;
        /** The conversions of earlier values, by the release introducing each. */
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

    // require a default the value's schema and JSON both take
    valueSchema.parse(defaultValue);
    schema.json().parse(defaultValue);

    // require a record value for a setting merging its keys
    const described = toJsonSchema(valueSchema);
    if (metadata.merge === "key" && (described["type"] !== "object" || "properties" in described)) {
        throw new TypeError(
            `setting ${definition.name} merges the keys of a value that is not a record`,
        );
    }

    return new Setting(owner, definition);
}
