import { getSharedLabel } from "../chunks/render-c0f21f7b.js";
export function getSettingsCatalog() {
    return [getSharedLabel("alpha"), getSharedLabel("beta"), getSharedLabel("settings")];
}

import { getSettingsSectionList } from "../chunks/render-c0f21f7b.js";
export function getSettingsFeatureSet() {
    return [...getSettingsSectionList(), "permissions-panel", "audit-log"];
}

import { renderPage } from "../chunks/render-c0f21f7b.js";
console.log(renderPage("settings", getSettingsCatalog(), getSettingsFeatureSet()));
