import { renderPage } from "./shared/render.ts";
import { getMarketingFeatureSet } from "./entry-local/marketing.ts";
import { getSharedCatalog } from "./shared/shared-catalog.ts";

console.log(renderPage("marketing", getSharedCatalog(), getMarketingFeatureSet()));
