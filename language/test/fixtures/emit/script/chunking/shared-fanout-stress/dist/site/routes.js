export function getRoutePrefix() {
    return "/workspace";
}

export function getPrimaryNavigation(activeSlug) {
    const slugs = ["overview", "admin", "docs", "reports"];
    return slugs.map((slug) => ({
        slug,
        href: `${getRoutePrefix()}/${slug}`,
        isActive: slug === activeSlug,
    }));
}

export function getCommonCards(prefix) {
    return [`${prefix}-summary`, `${prefix}-trend`, `${prefix}-activity`];
}

export function getInsightBadge(name) {
    return `badge-${name}`;
}

export function getSharedInsights(slug) {
    return [
        `${slug}:${getInsightBadge("alpha")}`,
        `${slug}:${getInsightBadge("beta")}`,
        `${slug}:${getInsightBadge("gamma")}`,
    ];
}

export function renderInsights(insights) {
    return insights.join(";");
}

export function renderShell(page) {
    const navigation = page.navigation.map(
        (item) => `${item.slug}:${item.href}:${item.isActive}`
    ).join("|");
    const cards = page.section.cards.join(",");
    return `${page.slug}:${navigation}:${page.section.heading}:${cards}:${renderInsights(
        page.insights
    )}`;
}
//# sourceMappingURL=./routes.js.map
