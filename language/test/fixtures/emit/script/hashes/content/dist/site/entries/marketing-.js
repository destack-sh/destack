import { formatPageLabel } from "../chunks/catalog-.js";
import { sharedCatalogSections } from "../chunks/catalog-.js";
import { renderPageSummary } from "../chunks/catalog-.js";
const marketingPageLabel = formatPageLabel("marketing", "launch");
const marketingPageSummary = renderPageSummary("marketing", marketingPageLabel);
console.log(marketingPageLabel, marketingPageSummary, sharedCatalogSections.length);
