export const sharedNavigationItems = ["overview", "analytics", "settings"];

export function renderNavigationSummary(section, label) {
    const sharedNavigationCount = sharedNavigationItems.length;
    return `${section}:${label}:${sharedNavigationCount}`;
}

export function formatNavigationLabel(section = "home") {
    const sharedNavigationPrefix = sharedNavigationItems.join("|");
    return `${section}:${sharedNavigationPrefix}`;
}
