import { formatNavigationLabel } from "../chunks/chunk-454b5c3d.js";
import { sharedNavigationItems } from "../chunks/chunk-454b5c3d.js";
import { renderNavigationSummary } from "../chunks/chunk-454b5c3d.js";
const settingsLabel = formatNavigationLabel("settings");
const settingsSummary = renderNavigationSummary("settings", settingsLabel);
console.log(settingsLabel, settingsSummary, sharedNavigationItems.length);
