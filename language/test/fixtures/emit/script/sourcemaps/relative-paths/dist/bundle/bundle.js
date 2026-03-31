export const currentSlug = "Emit Fixtures";

export function formatDocsSlug(slug) {
    return slug.replace(/\s+/g, "-").toLowerCase();
}

export function renderDocsPageRoute(slug) {
    const docsSlug = formatDocsSlug(slug);
    return `/docs/${docsSlug}`;
}

export function renderDocsCard(section, route) {
    return `${section}:${route}`;
}

const docsPageRoute = renderDocsPageRoute(currentSlug);
const docsCard = renderDocsCard("emit", docsPageRoute);
export = docsCard;
//# sourceMappingURL=./bundle.js.map
