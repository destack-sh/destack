import { getCommonCards } from "./chunk-2a084f36.js";
export function getOverviewModule() {
    return { heading: "overview", cards: [...getCommonCards("overview"), "overview-usage"] };
}

import { getSharedInsights } from "./chunk-2a084f36.js";
import { getPrimaryNavigation } from "./chunk-2a084f36.js";
export function getOverviewPage() {
    return {
        slug: "overview",
        navigation: getPrimaryNavigation("overview"),
        section: getOverviewModule(),
        insights: getSharedInsights("overview"),
    };
}

import { renderShell } from "./chunk-2a084f36.js";
console.log("app", renderShell(getOverviewPage()));
//# sourceMappingURL=./app.js.map
