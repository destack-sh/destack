export const overviewRoute = "courses-overview";
export const dashboardRoute = "courses-dashboard";
export const reportsRoute = "courses-reports";
export const settingsRoute = "courses-settings";
export function getOverviewRoute() {
    return overviewRoute;
}
export function getDashboardRoute() {
    return dashboardRoute;
}
export function getReportsRoute() {
    return reportsRoute;
}

export const courseDescription = "Course routes, lesson pages, and reporting panels.";
export function getPageTitle(lessonTitle) {
    const displayTitle = lessonTitle.replace(/-/g, " ");
    return `${displayTitle} | academy`;
}
export function getPageDescription(route) {
    return `${courseDescription} Route: ${route}.`;
}
export function getPageMetadata(route, lessonTitle) {
    const title = getPageTitle(lessonTitle);
    const description = getPageDescription(route);
    return { title, description };
}

export const overviewLessonTitle = "lesson-overview";
export const dashboardLessonTitle = "lesson-dashboard";
export const reportsLessonTitle = "lesson-reports";
export const settingsLessonTitle = "lesson-settings";
export function getOverviewLessonTitle() {
    return overviewLessonTitle;
}
export function getDashboardLessonTitle() {
    return dashboardLessonTitle;
}
export function getReportsLessonTitle() {
    return reportsLessonTitle;
}

export function getPanelHref(route) {
    return `/courses/${route}`;
}
export function getPanelLabel(route, title) {
    return `${title} | ${route}`;
}
export function getPanelImagePath(route) {
    return `/og/${route}`;
}
export function getPanelCard(route, title) {
    const href = getPanelHref(route);
    const label = getPanelLabel(route, title);
    const imagePath = getPanelImagePath(route);
    return { href, label, imagePath };
}
//# sourceMappingURL=./chunk-a88da36c.js.map
