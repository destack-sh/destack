import { defineSchema, schema } from "@destack/schema";
import { SettingReference } from "../setting/setting.ts";
import { SettingMode } from "../setting/mode.ts";

/** A setting value a stack places in its space. */
export const SpaceSetting = defineSchema(
    schema.object({
        /** The setting. */
        setting: SettingReference,
        /** The value. */
        value: schema.json(),
        /** How the value applies. */
        mode: SettingMode,
    }),
);
/** A setting value a stack places in its space. */
export type SpaceSetting = schema.Infer<typeof SpaceSetting>;
