import { formatPageLabel } from "../chunks/catalog-.js";
import { sharedCatalogSections } from "../chunks/catalog-.js";
import { renderPageSummary } from "../chunks/catalog-.js";
const docsPageLabel = formatPageLabel("docs", "guides");
const docsPageSummary = renderPageSummary("docs", docsPageLabel);
console.log(docsPageLabel, docsPageSummary, sharedCatalogSections.length);
