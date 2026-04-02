import { getCommonCards } from "./routes.js";
export function getDocsModule() {
    return { heading: "docs", cards: [...getCommonCards("docs"), "docs-guides"] };
}

import { getSharedInsights } from "./routes.js";
import { getPrimaryNavigation } from "./routes.js";
export function getDocsPage() {
    return {
        slug: "docs",
        navigation: getPrimaryNavigation("docs"),
        section: getDocsModule(),
        insights: getSharedInsights("docs"),
    };
}

import { renderShell } from "./routes.js";
console.log("docs", renderShell(getDocsPage()));
//# sourceMappingURL=./docs.js.map
