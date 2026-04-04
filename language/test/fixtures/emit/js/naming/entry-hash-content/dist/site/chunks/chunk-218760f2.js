export const sharedCatalogSections = ["guides", "examples", "api"];

export function formatPageLabel(page, section) {
    const sharedCatalogLabel = sharedCatalogSections.join(",");
    return `${page}:${section}:${sharedCatalogLabel}`;
}

export function renderPageSummary(page, label) {
    const sharedCatalogCount = sharedCatalogSections.length;
    return `${page}:${label}:${sharedCatalogCount}`;
}
