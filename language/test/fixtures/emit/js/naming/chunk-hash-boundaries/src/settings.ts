import { getSettingsFeatureSet } from "./entry-local/settings.ts";
import { renderPage } from "./shared/render.ts";
import { getSettingsCatalog } from "./shared/settings-catalog.ts";

console.log(renderPage("settings", getSettingsCatalog(), getSettingsFeatureSet()));
