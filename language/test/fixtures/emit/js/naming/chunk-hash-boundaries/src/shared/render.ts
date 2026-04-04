export function renderPage(
    slug: string,
    catalog: string[],
    features: string[],
) {
    return `${slug}:${catalog.join("|")}:${features.join("|")}`;
}
