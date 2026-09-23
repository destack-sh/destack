import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import { SettingMetadata, SettingScope } from "../declare/setting.ts";
import type { Setting } from "../setting/setting.ts";

/** The serializable declaration used by inspection and settings views. */
const description = SettingMetadata.extend({
    /** JSON Schema for the declared value. */
    schema: schema.record(schema.string(), schema.json()),
    /** The validated declaration default. */
    default: schema.json(),
});

/** The serializable value declaration and supported target refinements. */
export const SettingDescription = defineSchema(
    schema.discriminatedUnion("scope", [
        SettingScope.options[0].extend(description.shape),
        SettingScope.options[1].extend(description.shape),
        SettingScope.options[2].extend(description.shape),
    ]),
);
/** An inspected declaration retaining its package release. */
export type SettingDescription = schema.Infer<typeof SettingDescription>;

/** Describe an inert declaration without reading assignments. */
export function describeSetting(setting: Setting): SettingDescription {
    const { schema: valueSchema, ...declaration } = setting.declaration;

    return {
        ...declaration,
        default: schema.json().parse(declaration.default),
        schema: schema.record(schema.string(), schema.json()).parse(toJsonSchema(valueSchema)),
    };
}
