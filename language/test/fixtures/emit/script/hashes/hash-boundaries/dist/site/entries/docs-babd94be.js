import { getSharedSectionList } from "../chunks/render-c0f21f7b.js";
export function getDocsFeatureSet() {
    return [...getSharedSectionList(), "api-reference", "migration-guide"];
}

import { renderPage } from "../chunks/render-c0f21f7b.js";
import { getSharedCatalog } from "../chunks/shared-catalog-80f74087.js";
console.log(renderPage("docs", getSharedCatalog(), getDocsFeatureSet()));
