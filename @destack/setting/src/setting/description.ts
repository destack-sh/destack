import { JsonValue, schema, toJsonSchema } from "@destack/schema";
import type { Setting, SettingDescription } from "./setting.ts";

/** Describe a setting and its conversions, so every build reader converts earlier values. */
export function describeSetting(setting: Setting): SettingDescription {
    const { schema: valueSchema, ...definition } = setting.definition;

    return {
        ...definition,
        package: setting.package,
        default: schema.json().parse(definition.default),
        schema: JsonValue.object(toJsonSchema(valueSchema)),
    };
}
