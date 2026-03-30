export function getPageTitle(slug = "") {
    if(slug === "") {
        return "Overview";
    }
    if(slug === "github") {
        return "GitHub";
    }
    if(slug === "configuration") {
        return "Configuration";
    }
    return "Overview";
}
export function getPageHeading(slug = "") {
    return `${getPageTitle(slug)} Reference`;
}
export function getPageDescription(slug = "") {
    if(slug === "github") {
        return "GitHub API operations and webhook tooling";
    }
    if(slug === "configuration") {
        return "Workspace configuration and runtime settings";
    }
    return "Local API Emulation for CI and Sandboxes";
}
export function getNavigationLabel(slug = "") {
    return `${getPageTitle(slug)} Docs`;
}
//# sourceMappingURL=./page-metadata.js.map
