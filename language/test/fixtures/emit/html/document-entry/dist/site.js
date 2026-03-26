const pageSections = ["overview", "settings", "activity"];
const pageDescriptions = {
    overview: "Review workspace configuration and owner settings",
    settings: "Inspect editors, notifications, and access controls",
    activity: "Track job history, retries, and audit events",
};

export const pageTitle = "Workspace Overview";

export function normalizePageSlug(slug = "overview") {
    const normalizedSlug = pageSections.includes(slug) ? slug : "overview";

    return normalizedSlug;
}

export function getSectionBadge(slug = "overview") {
    const normalizedSlug = normalizePageSlug(slug);

    if (normalizedSlug === "activity") {
        return "live";
    }

    return "guide";
}

export function pageHeading(slug = "overview") {
    return `${pageTitle}: ${normalizePageSlug(slug)}`;
}

export function pageSummary(slug = "overview") {
    const normalizedSlug = normalizePageSlug(slug);

    return pageDescriptions[normalizedSlug];
}

export function pageSectionBadge(slug = "overview") {
    return getSectionBadge(slug);
}

export function pageSectionPath(slug = "overview") {
    return `/reference/${normalizePageSlug(slug)}`;
}
//# sourceMappingURL=./site.js.map
