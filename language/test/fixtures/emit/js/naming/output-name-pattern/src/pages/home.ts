import { formatNavigationLabel } from "../shared/labels.ts";
import { sharedNavigationItems } from "../shared/navigation.ts";
import { renderNavigationSummary } from "../shared/render.ts";

const homeLabel = formatNavigationLabel("home");
const homeSummary = renderNavigationSummary("home", homeLabel);

console.log(homeLabel, homeSummary, sharedNavigationItems.length);
