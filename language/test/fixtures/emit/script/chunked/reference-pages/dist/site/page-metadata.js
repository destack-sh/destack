export const pageTitles = {
    "": "Local API Emulation\nfor CI and Sandboxes",
    configuration: "Configuration",
    vercel: "Vercel API",
    github: "GitHub API",
    google: "Google API",
    authentication: "Authentication",
    architecture: "Architecture",
};
export const pageDescriptions = {
    configuration: "Configure providers and tokens",
    github: "Inspect GitHub routes and responses",
    architecture: "Understand the service model",
};
export const missingPageTitle = "Unknown Page";
export const missingPageDescription = "No page description available";

export const sectionLabels = {
    configuration: "Setup",
    github: "Providers",
    google: "Providers",
    authentication: "Security",
    architecture: "Reference",
};
export function getSectionLabel(slug) {
    const sectionLabel = sectionLabels[slug];
    if (!sectionLabel) {
        return "Overview";
    }
    return sectionLabel;
}

export function getPageTitle(slug) {
    const pageTitle = pageTitles[slug];
    if (!pageTitle) {
        return missingPageTitle;
    }
    return pageTitle;
}
export function getPageDescription(slug) {
    const pageDescription = pageDescriptions[slug];
    if (!pageDescription) {
        return missingPageDescription;
    }
    return pageDescription;
}
export function getPageHeading(slug) {
    const pageTitle = getPageTitle(slug);
    if (pageTitle === missingPageTitle) {
        return missingPageTitle;
    }
    return `Docs: ${pageTitle}`;
}
export function getNavigationLabel(slug) {
    const sectionLabel = getSectionLabel(slug);
    const pageTitle = getPageTitle(slug);
    return `${sectionLabel}: ${pageTitle}`;
}
//# sourceMappingURL=./page-metadata.js.map
