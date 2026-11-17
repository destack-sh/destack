import { type JsonValue, Edition } from "@destack/schema";
import { Address, Plan, type Comparator } from "@destack/resource";
import { unreachablePlan } from "@destack/resource/error";
import { SettingDescription } from "../setting/setting.ts";
import type { Setting } from "../setting/setting.ts";

/** Plan a setting's value change between two releases. */
export const compareSetting: Comparator = (before, after) => {
    // read both releases' settings and the releases declaring them
    const earlier = SettingDescription.parse(before.description);
    const later = SettingDescription.parse(after.description);
    const release = after.symbol.package.version;

    // refuse a conversion to an edition the earlier release has that the later one changes
    const target = Address.join("setting", later.name);
    const changed = Edition.changed(earlier.convert ?? {}, later.convert ?? {});
    if (changed.length > 0) {
        const from = before.symbol.package.version;
        const detail = `changes the conversion to edition ${changed.join(", ")} that release ${from} has: add a new edition`;
        throw unreachablePlan([{ target, detail }]);
    }

    return Plan.of({
        target,
        before: earlier.schema,
        after: later.schema,
        release,
        compatibility: "backward",
        isConverted: Edition.of(later.convert ?? {}) > Edition.of(earlier.convert ?? {}),
    });
};

/** List a setting's term: its name, with its scope. */
export function settingVocabulary(setting: Setting): Record<string, JsonValue> {
    return { [setting.name]: { scope: setting.definition.scope } };
}
