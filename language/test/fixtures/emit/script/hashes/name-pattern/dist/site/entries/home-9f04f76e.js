import { formatNavigationLabel } from "../chunks/navigation-cfe7e839.js";
import { sharedNavigationItems } from "../chunks/navigation-cfe7e839.js";
import { renderNavigationSummary } from "../chunks/navigation-cfe7e839.js";
const homeLabel = formatNavigationLabel("home");
const homeSummary = renderNavigationSummary("home", homeLabel);
console.log(homeLabel, homeSummary, sharedNavigationItems.length);
