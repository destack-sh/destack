import { schema, toJsonSchema, Version } from "@destack/schema";
import { Expression } from "@destack/db";
import { ModuleMetadata, Package } from "@destack/package";
import { Setting, SettingMetadata, SettingScope } from "../setting/setting.ts";

/** A setting as authored, typed by whether it has a default. */
export type SettingDefinition<
    Value extends schema.Schema = schema.Schema,
    Default extends SettingDefault<Value> = SettingDefault<Value>,
> = SettingFields<Value> & Default;

/** The fields of a setting beside its default. */
type SettingFields<Value extends schema.Schema> = schema.Infer<typeof SettingMetadata> &
    SettingScope & {
        /** The value schema. */
        readonly schema: Value;
        /** The conversions of earlier values, by the release introducing each. */
        readonly convert?: Readonly<Record<Version, Expression>>;
    };

/** What applies where no scope placed a value: a default, or a requirement to place one. */
export type SettingDefault<Value extends schema.Schema = schema.Schema> =
    | {
          /** The value when no placed value applies. */
          readonly default: schema.Infer<Value>;
          /** No requirement beside a default. */
          readonly required?: never;
      }
    | {
          /** Whether each scope must place a value, as the setting has no default. */
          readonly required: true;
          /** No default beside a requirement. */
          readonly default?: never;
      };

/** The default branch a declaration takes, its default typed by the value's schema. */
type DefaultOf<Value extends schema.Schema, Declared> = Extract<
    SettingDefault<Value>,
    Declared extends { readonly default: unknown }
        ? { readonly default: unknown }
        : { readonly required: true }
>;

/** Declare a typed setting, typed by whether it has a default. */
export function defineSetting<
    Value extends schema.Schema,
    const Declared extends
        | { readonly default: unknown; readonly required?: never }
        | { readonly required: true; readonly default?: never },
>(
    definition: SettingFields<Value> & DefaultOf<Value, Declared> & Declared,
    module?: ModuleMetadata,
): Setting<Value, DefaultOf<Value, Declared>> {
    // stamp the declaring package
    const owner = Package.parse(ModuleMetadata.require(module, "defineSetting").package);

    // validate the serializable fields
    const { schema: valueSchema, scope, overrides, convert, ...declared } = definition;
    const metadata = SettingMetadata.strip().parse(declared);
    SettingScope.parse({ scope, overrides });

    // require conversions keyed by releases up to the declaring one
    Version.requireUpTo(convert ?? {}, owner.version, definition.name);

    // require exactly one of a default and a requirement, and a default the value's schema and JSON both take
    if ("default" in definition === "required" in definition) {
        throw new TypeError(`setting ${definition.name} needs either a default or required`);
    }
    if ("default" in definition) {
        valueSchema.parse(definition.default);
        schema.json().parse(definition.default);
    }

    // require a record value for a setting merging its keys
    const described = toJsonSchema(valueSchema);
    if (metadata.merge === "key" && (described["type"] !== "object" || "properties" in described)) {
        throw new TypeError(
            `setting ${definition.name} merges keys of a value that is not a record`,
        );
    }

    return new Setting<Value, DefaultOf<Value, Declared>>(owner, definition);
}
