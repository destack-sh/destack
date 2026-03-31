import { buildPreviewSection } from "./paths.ts";

/** Build one preview source url string from `import.meta.url`. */
export function buildPreviewSourceUrl(section = "overview") {
    const previewSection = buildPreviewSection(section);

    return `${import.meta.url}#${previewSection}`;
}

/** Build one preview label string from `import.meta.url`. */
export function buildPreviewLabel(section = "overview") {
    const previewSourceUrl = buildPreviewSourceUrl(section);

    return `preview:${previewSourceUrl}`;
}
