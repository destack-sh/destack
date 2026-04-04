import { formatNavigationLabel } from "../chunks/chunk-454b5c3d.js";
import { sharedNavigationItems } from "../chunks/chunk-454b5c3d.js";
import { renderNavigationSummary } from "../chunks/chunk-454b5c3d.js";
const homeLabel = formatNavigationLabel("home");
const homeSummary = renderNavigationSummary("home", homeLabel);
console.log(homeLabel, homeSummary, sharedNavigationItems.length);
