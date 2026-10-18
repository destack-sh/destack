import { type JsonValue, Version } from "@destack/schema";
import { Address, Plan, type Comparator } from "@destack/resource";
import { SettingDescription } from "../setting/setting.ts";
import type { Setting } from "../setting/setting.ts";

/** Plan a setting's value change between two releases. */
export const compareSetting: Comparator = (before, after) => {
    // read both releases' settings and the releases declaring them
    const earlier = SettingDescription.parse(before.description);
    const later = SettingDescription.parse(after.description);
    const release = after.symbol.package.version;

    return Plan.values({
        target: Address.join("setting", later.name),
        before: earlier.schema,
        after: later.schema,
        release,
        compatibility: "backward",
        isConverted:
            Version.between(later.convert ?? {}, before.symbol.package.version, release).length > 0,
    });
};

/** List a setting's term: its name, with its scope. */
export function settingVocabulary(setting: Setting): Record<string, JsonValue> {
    return { [setting.name]: { scope: setting.definition.scope } };
}
