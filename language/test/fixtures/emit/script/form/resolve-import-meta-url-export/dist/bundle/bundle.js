export function buildPreviewSection(section = "overview") {
    return `preview/${section}`;
}

export function buildPreviewSourceUrl(section = "overview") {
    const previewSection = buildPreviewSection(section);
    return `${import.meta.url}#${previewSection}`;
}
export function buildPreviewLabel(section = "overview") {
    const previewSourceUrl = buildPreviewSourceUrl(section);
    return `preview:${previewSourceUrl}`;
}

export const url = import.meta.url;
export const meta = import.meta;
export const previewSourceUrl = buildPreviewSourceUrl("cards");
export const previewLabel = buildPreviewLabel("cards");
//# sourceMappingURL=./bundle.js.map
