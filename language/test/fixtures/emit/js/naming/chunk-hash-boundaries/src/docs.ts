import { renderPage } from "./shared/render.ts";
import { getDocsFeatureSet } from "./entry-local/docs.ts";
import { getSharedCatalog } from "./shared/shared-catalog.ts";

console.log(renderPage("docs", getSharedCatalog(), getDocsFeatureSet()));
