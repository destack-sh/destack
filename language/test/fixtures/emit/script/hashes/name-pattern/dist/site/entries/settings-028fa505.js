import { formatNavigationLabel } from "../chunks/navigation-cfe7e839.js";
import { sharedNavigationItems } from "../chunks/navigation-cfe7e839.js";
import { renderNavigationSummary } from "../chunks/navigation-cfe7e839.js";
const settingsLabel = formatNavigationLabel("settings");
const settingsSummary = renderNavigationSummary("settings", settingsLabel);
console.log(settingsLabel, settingsSummary, sharedNavigationItems.length);
