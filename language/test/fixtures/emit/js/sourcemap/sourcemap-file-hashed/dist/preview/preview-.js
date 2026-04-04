export function formatGuidePath(slug) {
    return `/guides/${slug}`;
}

export function renderGuideSummary(slug, section) {
    const guidePath = formatGuidePath(slug);
    return `${guidePath}#${section}`;
}

export const currentGuide = { slug: "emit-fixtures", section: "sourcemaps" };

export function renderGuideCard(title, summary) {
    return `${title}:${summary}`;
}

const guideSummary = renderGuideSummary(currentGuide.slug, currentGuide.section);
const guideCard = renderGuideCard("guides", guideSummary);
console.log(guideCard);
//# sourceMappingURL=./preview-.js.map
