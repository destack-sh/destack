import { defineSchema, schema } from "@destack/schema";
import { type Setting, SettingReference } from "../setting/setting.ts";
import { SettingMode } from "../setting/mode.ts";
import type {} from "@destack/space/declare";

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

/** Add the settings collection to stacks. */
declare module "@destack/space/declare" {
    interface SpaceDocument {
        /** The setting values the stack places in its space. */
        readonly settings?: Readonly<
            Record<
                string,
                Omit<SpaceSetting, "setting"> & { readonly setting: Setting | SettingReference }
            >
        >;
    }
}
