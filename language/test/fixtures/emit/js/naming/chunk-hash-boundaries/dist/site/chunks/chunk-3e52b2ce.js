export function renderPage(slug, catalog, features) {
    return `${slug}:${catalog.join("|")}:${features.join("|")}`;
}

export function getSharedSectionList() {
    return ["overview", "activity", "health"];
}
export function getSettingsSectionList() {
    return ["overview", "activity", "policies"];
}

export function getSharedLabel(name) {
    return `shared-${name}`;
}
