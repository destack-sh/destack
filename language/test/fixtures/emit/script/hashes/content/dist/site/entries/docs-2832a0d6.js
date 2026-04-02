import { formatPageLabel } from "../chunks/catalog-10e2d02d.js";
import { sharedCatalogSections } from "../chunks/catalog-10e2d02d.js";
import { renderPageSummary } from "../chunks/catalog-10e2d02d.js";
const docsPageLabel = formatPageLabel("docs", "guides");
const docsPageSummary = renderPageSummary("docs", docsPageLabel);
console.log(docsPageLabel, docsPageSummary, sharedCatalogSections.length);
