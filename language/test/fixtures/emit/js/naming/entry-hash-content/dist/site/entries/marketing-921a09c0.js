import { formatPageLabel } from "../chunks/chunk-218760f2.js";
import { sharedCatalogSections } from "../chunks/chunk-218760f2.js";
import { renderPageSummary } from "../chunks/chunk-218760f2.js";
const marketingPageLabel = formatPageLabel("marketing", "launch");
const marketingPageSummary = renderPageSummary("marketing", marketingPageLabel);
console.log(marketingPageLabel, marketingPageSummary, sharedCatalogSections.length);
