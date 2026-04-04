export const previewBaseUrl = import.meta.url;
export const previewMeta = import.meta;
export function buildPreviewHref(name) {
    return `${name}.html`;
}

export function escapePreviewTitle(title) {
    return title.replaceAll("&", "&amp;").replaceAll("\\\"", "&quot;");
}
export function renderPreviewSummary(name, route) {
    return `${name}:${route}`;
}

export * from "external";
export const escapedTitle = escapePreviewTitle(`Preview "cards" & docs`);
export const previewSummary = renderPreviewSummary("cards", "overview");
export const previewHref = buildPreviewHref("cards");
export const previewSource = import.meta.url;
export const previewMetaSnapshot = previewMeta;
export const previewBase = previewBaseUrl;
//# sourceMappingURL=./bundle.js.map
