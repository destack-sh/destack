import { getCommonCards } from "./routes.js";
export function getReportsModule() {
    return { heading: "reports", cards: [...getCommonCards("reports"), "reports-cohort"] };
}

import { getSharedInsights } from "./routes.js";
import { getPrimaryNavigation } from "./routes.js";
export function getReportsPage() {
    return {
        slug: "reports",
        navigation: getPrimaryNavigation("reports"),
        section: getReportsModule(),
        insights: getSharedInsights("reports"),
    };
}

import { renderShell } from "./routes.js";
console.log("reports", renderShell(getReportsPage()));
//# sourceMappingURL=./reports.js.map
