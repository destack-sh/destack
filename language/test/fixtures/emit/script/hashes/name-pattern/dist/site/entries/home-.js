import { formatNavigationLabel } from "../chunks/navigation-.js";
import { sharedNavigationItems } from "../chunks/navigation-.js";
import { renderNavigationSummary } from "../chunks/navigation-.js";
const homeLabel = formatNavigationLabel("home");
const homeSummary = renderNavigationSummary("home", homeLabel);
console.log(homeLabel, homeSummary, sharedNavigationItems.length);
