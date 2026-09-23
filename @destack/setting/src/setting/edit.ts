import { defineSchema, schema } from "@destack/schema";
import { Creation } from "@destack/service/procedure";
import { SettingReference } from "./setting.ts";
import { SettingTarget } from "./target.ts";

/** A conditional assignment edit with an identity retained across retries. */
export const SettingEdit = defineSchema(
    Creation.extend({
        /** The configured declaration. */
        setting: SettingReference,
        /** The exact person or runtime selection. */
        target: SettingTarget,
        /** The observed revision; null requires that no assignment exists. */
        expectedRevision: schema.uuid().nullable(),
    }),
);
/** A conditional assignment edit. */
export type SettingEdit = schema.Infer<typeof SettingEdit>;
