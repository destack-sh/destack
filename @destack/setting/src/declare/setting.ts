import { schema, toJsonSchema, Version } from "@destack/schema";
import { Expression } from "@destack/db";
import { ModuleMetadata, PackageVersion } from "@destack/package";
import { Setting, SettingMetadata, SettingScope } from "../setting/setting.ts";

/** A setting as declared, typed by whether it has a default. */
export type SettingDefinition<
    Value extends schema.Schema = schema.Schema,
    Default extends SettingDefault<Value> = SettingDefault<Value>,
> = SettingFields<Value> & Default;

/** The fields of a setting beside its default, its scope and when it applies. */
type SettingCommon<Value extends schema.Schema> = Omit<
    schema.Infer<typeof SettingMetadata>,
    "apply"
> & {
    /** The value schema. */
    readonly schema: Value;
    /** The conversions of earlier values, by the release introducing each. */
    readonly convert?: Readonly<Record<Version, Expression>>;
};

/** When a consumer applies a changed value. */
type SettingApply = schema.Infer<typeof SettingMetadata>["apply"];

/** The fields of a setting beside its default. */
type SettingFields<Value extends schema.Schema> = SettingCommon<Value> &
    SettingScope & {
        /** When a consumer applies a changed value. */
        readonly apply: SettingApply;
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

/** A setting's scope and overrides as authored, no overrides when absent. */
type SettingScopeOptions = SettingScope extends infer Each
    ? Each extends { readonly scope: infer Scope; readonly overrides: infer Overrides }
        ? {
              /** Where the setting's values live. */
              readonly scope: Scope;
              /** The overrides the setting permits, none when absent. */
              readonly overrides?: Overrides;
          }
        : never
    : never;

/** A setting as authored: applied immediately and without overrides when it leaves them out. */
type SettingOptions<Value extends schema.Schema> = SettingCommon<Value> &
    SettingScopeOptions & {
        /** When a consumer applies a changed value, immediately when absent. */
        readonly apply?: SettingApply;
    };

/** Declare a typed setting, typed by whether it has a default. */
export function defineSetting<Value extends schema.Schema>(
    options: SettingOptions<Value> &
        Extract<SettingDefault<NoInfer<Value>>, { readonly default: unknown }>,
    module?: ModuleMetadata,
): Setting<Value, Extract<SettingDefault<Value>, { readonly default: unknown }>>;
export function defineSetting<Value extends schema.Schema>(
    options: SettingOptions<Value> &
        Extract<SettingDefault<NoInfer<Value>>, { readonly required: true }>,
    module?: ModuleMetadata,
): Setting<Value, Extract<SettingDefault<Value>, { readonly required: true }>>;
/** Declare a setting, its type computed from its options by the signatures above. */
export function defineSetting(
    options: SettingOptions<schema.Schema> & SettingDefault,
    module?: ModuleMetadata,
): Setting {
    // stamp the declaring package and fill the defaults the options leave out
    const owner = PackageVersion.parse(ModuleMetadata.require(module, "defineSetting").package);
    const { scope, overrides = [], apply = "immediate", ...common } = options;
    const placement = SettingScope.parse({ scope, overrides });
    const definition: SettingDefinition = { ...common, ...placement, apply };

    // validate the serializable fields
    const { schema: valueSchema, convert } = definition;
    const metadata = SettingMetadata.strip().parse(definition);

    // require conversions keyed by releases up to the declaring one
    Version.requireUpTo(convert ?? {}, owner.version, definition.name);

    // require exactly one of a default and a requirement, and a default the value's schema and JSON both take
    if ("default" in definition === "required" in definition) {
        throw new TypeError(`setting '${definition.name}' needs either a default or required`);
    }
    if ("default" in definition) {
        valueSchema.parse(definition.default);
        schema.json().parse(definition.default);
    }

    // require a record value for a setting merging its keys
    const described = toJsonSchema(valueSchema);
    if (metadata.merge === "key" && (described["type"] !== "object" || "properties" in described)) {
        throw new TypeError(
            `setting '${definition.name}' merges keys of a value that is not a record`,
        );
    }

    return new Setting(owner, definition);
}
