import { formatNavigationLabel } from "../chunks/navigation-.js";
import { sharedNavigationItems } from "../chunks/navigation-.js";
import { renderNavigationSummary } from "../chunks/navigation-.js";
const settingsLabel = formatNavigationLabel("settings");
const settingsSummary = renderNavigationSummary("settings", settingsLabel);
console.log(settingsLabel, settingsSummary, sharedNavigationItems.length);
