import { formatPageLabel } from "./shared/label.ts";
import { sharedCatalogSections } from "./shared/catalog.ts";
import { renderPageSummary } from "./shared/render/page.ts";

const marketingPageLabel = formatPageLabel("marketing", "launch");
const marketingPageSummary = renderPageSummary("marketing", marketingPageLabel);

console.log(
    marketingPageLabel,
    marketingPageSummary,
    sharedCatalogSections.length,
);
