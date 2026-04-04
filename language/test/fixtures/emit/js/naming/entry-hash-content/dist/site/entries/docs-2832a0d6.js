import { formatPageLabel } from "../chunks/chunk-218760f2.js";
import { sharedCatalogSections } from "../chunks/chunk-218760f2.js";
import { renderPageSummary } from "../chunks/chunk-218760f2.js";
const docsPageLabel = formatPageLabel("docs", "guides");
const docsPageSummary = renderPageSummary("docs", docsPageLabel);
console.log(docsPageLabel, docsPageSummary, sharedCatalogSections.length);
