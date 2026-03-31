import { getSettingsSectionList } from "./sections.ts";

export function getSettingsFeatureSet() {
    return [...getSettingsSectionList(), "permissions-panel", "audit-log"];
}
