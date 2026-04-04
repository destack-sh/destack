function formatDocsSlug(slug) {
    return slug.replace(/\s+/g, "-").toLowerCase();
}

function renderDocsPageRoute(slug) {
    const docsSlug = formatDocsSlug(slug);
    return `/docs/${docsSlug}`;
}

function renderDocsCard(section, route) {
    return `${section}:${route}`;
}

const currentSlug = "Emit Fixtures";
const docsPageRoute = renderDocsPageRoute(currentSlug);
const docsCard = renderDocsCard("emit", docsPageRoute);
export default docsCard;
//# sourceMappingURL=./bundle.js.map
