const pageSections = ["overview", "settings", "activity"];
export function normalizePageSlug(slug = "overview") {
    return pageSections.includes(slug)?slug:"overview";
}
export function getSectionBadge(slug = "overview") {
    const normalizedSlug = normalizePageSlug(slug);
    if(normalizedSlug === "activity") {
        return "live";
    }
    return "guide";
}

export const pageTitle = "Workspace Overview";
export function pageHeading(slug = "overview") {
    return `${pageTitle}: ${normalizePageSlug(slug)}`;
}
export function getPageSummary(slug = "overview") {
    const normalizedSlug = normalizePageSlug(slug);
    if(normalizedSlug == "settings") {
        return "Inspect editors, notifications, and access controls";
    }
    if(normalizedSlug == "activity") {
        return "Track job history, retries, and audit events";
    }
    return "Review workspace configuration and owner settings";
}

export { pageTitle, pageHeading };
export function pageSummary(slug = "overview") {
    return getPageSummary(slug);
}
export function pageSectionBadge(slug = "overview") {
    return getSectionBadge(slug);
}
export function pageSectionPath(slug = "overview") {
    return `/reference/${normalizePageSlug(slug)}`;
}
//# sourceMappingURL=./site.js.map
