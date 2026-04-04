import { getSharedSectionList } from "../chunks/chunk-3e52b2ce.js";
export function getDocsFeatureSet() {
    return [...getSharedSectionList(), "api-reference", "migration-guide"];
}

import { renderPage } from "../chunks/chunk-3e52b2ce.js";
import { getSharedCatalog } from "../chunks/shared-catalog-80f74087.js";
console.log(renderPage("docs", getSharedCatalog(), getDocsFeatureSet()));
