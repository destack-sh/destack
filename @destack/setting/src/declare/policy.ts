import { defineSchema, identifier, schema } from "@destack/schema";
import { SettingReference, type Setting } from "../setting/setting.ts";
import { SettingAuthority } from "../setting/target.ts";

/** Desired policy contents; applying them requires existing administrative permission. */
export const SettingPolicyDefinition = defineSchema(
    schema.object({
        /** The declaration governed by the policy. */
        setting: SettingReference,
        /** The administering account, space or host. */
        authority: SettingAuthority,
        /** An optional receiving installation within the administered scope. */
        installationId: identifier("installation").optional(),
        /** Recommended values remain overridable; required values must agree. */
        mode: schema.enum(["recommended", "required"]),
        /** The complete value checked against the consuming declaration. */
        value: schema.json(),
    }),
);
/** Source-authored policy contents. */
export type SettingPolicyDefinition = schema.Infer<typeof SettingPolicyDefinition>;

/** Describe a typed recommendation or required value under an administrative authority. */
export function defineSettingPolicy<Value extends schema.Schema>(
    setting: Setting<Value>,
    policy: Omit<SettingPolicyDefinition, "setting" | "value">,
    value: schema.Infer<Value>,
): SettingPolicyDefinition {
    return SettingPolicyDefinition.parse({
        ...policy,
        setting: setting.reference,
        value: setting.declaration.schema.parse(value),
    });
}
