const pageDescriptions = {
    "": "Local API Emulation for CI and Sandboxes",
    github: "GitHub API operations and webhook tooling",
    configuration: "Workspace configuration and runtime settings",
};

export function getPageTitle(slug = "") {
    if (slug === "") {
        return "Overview";
    }

    return slug[0].toUpperCase() + slug.slice(1);
}

export function getPageHeading(slug = "") {
    return `${getPageTitle(slug)} Reference`;
}

export function getPageDescription(slug = "") {
    return pageDescriptions[slug] ?? pageDescriptions[""];
}

export function getNavigationLabel(slug = "") {
    return `${getPageTitle(slug)} Docs`;
}
//# sourceMappingURL=./page-metadata.js.map
