import { JsonValue, schema, toJsonSchema } from "@destack/schema";
import { type Setting, type SettingDescription, SettingMetadata, SettingScope } from "./setting.ts";

/** Describe a setting and its conversions, so every build reader converts earlier values. */
export function describeSetting(setting: Setting): SettingDescription {
    const { definition } = setting;

    return {
        ...SettingMetadata.strip().parse(definition),
        ...SettingScope.parse({ scope: definition.scope, overrides: definition.overrides }),
        ...(definition.convert === undefined ? {} : { convert: definition.convert }),
        package: setting.package,
        ...("default" in definition
            ? { default: schema.json().parse(definition.default) }
            : { required: true as const }),
        schema: JsonValue.object(toJsonSchema(definition.schema)),
    };
}
