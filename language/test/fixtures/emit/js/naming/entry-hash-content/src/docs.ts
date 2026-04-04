import { formatPageLabel } from "./shared/label.ts";
import { sharedCatalogSections } from "./shared/catalog.ts";
import { renderPageSummary } from "./shared/render/page.ts";

const docsPageLabel = formatPageLabel("docs", "guides");
const docsPageSummary = renderPageSummary("docs", docsPageLabel);

console.log(docsPageLabel, docsPageSummary, sharedCatalogSections.length);
