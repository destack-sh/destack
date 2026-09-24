import { declaringModule, type ModuleMetadata } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import { Setting, SettingName } from "../setting/setting.ts";

/** Supported base scopes and their permitted refinements. */
export const SettingScope = defineSchema(
    schema.discriminatedUnion("scope", [
        schema.object({
            /** Personal values resolved for the represented user. */
            scope: schema.literal("user"),
            /** Refinements enabled for this declaration. */
            overrides: schema.array(schema.enum(["package", "space", "installation", "device"])),
        }),
        schema.object({
            /** Shared values resolved for the receiving space. */
            scope: schema.literal("space"),
            /** Installation-specific configuration, when supported. */
            overrides: schema.array(schema.literal("installation")),
        }),
        schema.object({
            /** Values retained on one execution host. */
            scope: schema.literal("host"),
            /** Host settings have no implicit child scope. */
            overrides: schema.tuple([]),
        }),
    ]),
);
/** Supported base scope and refinements. */
export type SettingScope = schema.Infer<typeof SettingScope>;

/** Setting metadata shared by authoring and inspection. */
export const SettingMetadata = defineSchema(
    schema.object({
        /** The stable package-local name. */
        name: SettingName,
        /** The label used in settings views. */
        title: schema.string().min(1),
        /** The behavior controlled by the value. */
        description: schema.string().min(1),
        /** An optional presentation group within the declaring package. */
        group: schema.string().min(1).optional(),
        /** When a consumer applies a changed effective value. */
        apply: schema.enum(["immediate", "restart"]),
        /** Migration guidance for a declaration retained for compatibility. */
        deprecated: schema.string().min(1).optional(),
    }),
);

/** A setting as authored: metadata, scopes, the native value schema and its default. */
export type SettingDefinition<Value extends schema.Schema = schema.Schema> = schema.Infer<
    typeof SettingMetadata
> &
    SettingScope & {
        /** The native validator shared by writes and consumers. */
        readonly schema: Value;
        /** The complete value used when no assignment or policy supplies one. */
        readonly default: schema.Infer<Value>;
    };

/** Declare a typed setting without reading or writing assignments. */
export function defineSetting<Value extends schema.Schema>(
    definition: SettingDefinition<Value>,
    module?: ModuleMetadata,
): Setting<Value> {
    // stamp the declaring package supplied by the module transform
    const owner = declaringModule(module, "defineSetting").package;

    // validate serializable metadata independently of the native schema
    const {
        schema: valueSchema,
        default: defaultValue,
        scope,
        overrides,
        ...metadata
    } = definition;
    SettingMetadata.parse(metadata);
    SettingScope.parse({ scope, overrides });

    // require a declarative value schema and a valid JSON default
    defineSchema(valueSchema);
    valueSchema.parse(defaultValue);
    schema.json().parse(defaultValue);

    return new Setting(owner, definition);
}
