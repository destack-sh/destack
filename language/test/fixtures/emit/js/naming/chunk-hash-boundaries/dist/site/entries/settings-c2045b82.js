import { getSharedLabel } from "../chunks/chunk-3e52b2ce.js";
export function getSettingsCatalog() {
    return [getSharedLabel("alpha"), getSharedLabel("beta"), getSharedLabel("settings")];
}

import { getSettingsSectionList } from "../chunks/chunk-3e52b2ce.js";
export function getSettingsFeatureSet() {
    return [...getSettingsSectionList(), "permissions-panel", "audit-log"];
}

import { renderPage } from "../chunks/chunk-3e52b2ce.js";
console.log(renderPage("settings", getSettingsCatalog(), getSettingsFeatureSet()));
