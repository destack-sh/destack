import { defineSchema, schema } from "@destack/schema";

/** The ways a value applies: set in its scope, or recommended or required by an enclosing one. */
export const SETTING_MODES = ["set", "recommend", "require"] as const;

/** How a value applies. */
export const SettingMode = defineSchema(schema.enum(SETTING_MODES));
/** How a value applies. */
export type SettingMode = schema.Infer<typeof SettingMode>;
