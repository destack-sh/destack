import { defineSchema, schema } from "@destack/schema";
import { SettingReference, type Setting } from "../setting/setting.ts";
import { SettingTarget } from "../setting/target.ts";

/** Desired assignment contents; reconciliation supplies identity and provenance. */
export const SettingAssignmentDefinition = defineSchema(
    schema.object({
        /** The declaration configured by the source. */
        setting: SettingReference,
        /** The person or runtime selected by the source. */
        target: SettingTarget,
        /** The value checked against the selected package declaration. */
        value: schema.json(),
    }),
);
/** Source-authored assignment contents. */
export type SettingAssignmentDefinition = schema.Infer<typeof SettingAssignmentDefinition>;

/** Describe an assignment using the imported setting's inferred value type. */
export function defineSettingAssignment<Value extends schema.Schema>(
    setting: Setting<Value>,
    target: SettingTarget,
    value: schema.Infer<Value>,
): SettingAssignmentDefinition {
    setting.assertTarget(target);

    return SettingAssignmentDefinition.parse({
        setting: setting.reference,
        target,
        value: setting.definition.schema.parse(value),
    });
}
