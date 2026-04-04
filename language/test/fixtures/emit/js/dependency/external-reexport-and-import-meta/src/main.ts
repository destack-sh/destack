export * from "external";

import { escapePreviewTitle, renderPreviewSummary } from "./preview.ts";
import { buildPreviewHref, previewBaseUrl, previewMeta } from "./preview-meta.ts";

export const escapedTitle = escapePreviewTitle(`Preview "cards" & docs`);
export const previewSummary = renderPreviewSummary("cards", "overview");
export const previewHref = buildPreviewHref("cards");
export const previewSource = import.meta.url;
export const previewMetaSnapshot = previewMeta;
export const previewBase = previewBaseUrl;
