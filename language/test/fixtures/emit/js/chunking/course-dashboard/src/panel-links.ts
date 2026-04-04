/** Build the href for a course panel route. */
export function getPanelHref(route: string) {
    return `/courses/${route}`;
}

/** Build the label for a course panel card. */
export function getPanelLabel(route: string, title: string) {
    return `${title} | ${route}`;
}

/** Build the image path for a course panel route. */
export function getPanelImagePath(route: string) {
    return `/og/${route}`;
}

/** Build the display model for a course panel card. */
export function getPanelCard(route: string, title: string) {
    const href = getPanelHref(route);
    const label = getPanelLabel(route, title);
    const imagePath = getPanelImagePath(route);

    return {
        href,
        label,
        imagePath,
    };
}
