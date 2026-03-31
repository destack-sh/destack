import { formatNavigationLabel } from "../shared/labels.ts";
import { sharedNavigationItems } from "../shared/navigation.ts";
import { renderNavigationSummary } from "../shared/render.ts";

const settingsLabel = formatNavigationLabel("settings");
const settingsSummary = renderNavigationSummary("settings", settingsLabel);

console.log(settingsLabel, settingsSummary, sharedNavigationItems.length);
