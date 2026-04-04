import { getCommonCards } from "./chunk-2a084f36.js";
export function getDocsModule() {
    return { heading: "docs", cards: [...getCommonCards("docs"), "docs-guides"] };
}

import { getSharedInsights } from "./chunk-2a084f36.js";
import { getPrimaryNavigation } from "./chunk-2a084f36.js";
export function getDocsPage() {
    return {
        slug: "docs",
        navigation: getPrimaryNavigation("docs"),
        section: getDocsModule(),
        insights: getSharedInsights("docs"),
    };
}

import { renderShell } from "./chunk-2a084f36.js";
console.log("docs", renderShell(getDocsPage()));
//# sourceMappingURL=./docs.js.map
