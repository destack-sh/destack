import { getCommonCards } from "./routes.js";
export function getOverviewModule() {
    return { heading: "overview", cards: [...getCommonCards("overview"), "overview-usage"] };
}

import { getSharedInsights } from "./routes.js";
import { getPrimaryNavigation } from "./routes.js";
export function getOverviewPage() {
    return {
        slug: "overview",
        navigation: getPrimaryNavigation("overview"),
        section: getOverviewModule(),
        insights: getSharedInsights("overview"),
    };
}

import { renderShell } from "./routes.js";
console.log("app", renderShell(getOverviewPage()));
//# sourceMappingURL=./app.js.map
