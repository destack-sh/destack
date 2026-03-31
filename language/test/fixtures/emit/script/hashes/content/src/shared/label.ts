import { sharedCatalogSections } from "./catalog.ts";

/** Format one entry label from entry-local content and shared content. */
export function formatPageLabel(page: string, section: string) {
    const sharedCatalogLabel = sharedCatalogSections.join(",");

    return `${page}:${section}:${sharedCatalogLabel}`;
}
