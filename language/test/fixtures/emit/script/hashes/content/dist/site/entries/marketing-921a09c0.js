import { formatPageLabel } from "../chunks/catalog-10e2d02d.js";
import { sharedCatalogSections } from "../chunks/catalog-10e2d02d.js";
import { renderPageSummary } from "../chunks/catalog-10e2d02d.js";
const marketingPageLabel = formatPageLabel("marketing", "launch");
const marketingPageSummary = renderPageSummary("marketing", marketingPageLabel);
console.log(marketingPageLabel, marketingPageSummary, sharedCatalogSections.length);
