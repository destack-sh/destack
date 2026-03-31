import { sharedCatalogSections } from "../catalog.ts";

/** Render one page summary string from the shared catalog state. */
export function renderPageSummary(page: string, label: string) {
    const sharedCatalogCount = sharedCatalogSections.length;

    return `${page}:${label}:${sharedCatalogCount}`;
}
