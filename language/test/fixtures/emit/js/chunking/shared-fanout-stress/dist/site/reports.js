import { getCommonCards } from "./chunk-2a084f36.js";
export function getReportsModule() {
    return { heading: "reports", cards: [...getCommonCards("reports"), "reports-cohort"] };
}

import { getSharedInsights } from "./chunk-2a084f36.js";
import { getPrimaryNavigation } from "./chunk-2a084f36.js";
export function getReportsPage() {
    return {
        slug: "reports",
        navigation: getPrimaryNavigation("reports"),
        section: getReportsModule(),
        insights: getSharedInsights("reports"),
    };
}

import { renderShell } from "./chunk-2a084f36.js";
console.log("reports", renderShell(getReportsPage()));
//# sourceMappingURL=./reports.js.map
