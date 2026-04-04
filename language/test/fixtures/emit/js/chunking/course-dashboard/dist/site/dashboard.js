import { getPageMetadata } from "./chunk-a88da36c.js";
import { getDashboardRoute, getReportsRoute } from "./chunk-a88da36c.js";
import { getDashboardLessonTitle, getReportsLessonTitle } from "./chunk-a88da36c.js";
import { getPanelCard } from "./chunk-a88da36c.js";
const dashboardRouteName = getDashboardRoute();
const dashboardTitle = getDashboardLessonTitle();
const dashboardReportsRoute = getReportsRoute();
const dashboardReportsTitle = getReportsLessonTitle();
const dashboardPanel = getPanelCard(dashboardRouteName, dashboardTitle);
const dashboardPage = getPageMetadata(dashboardRouteName, dashboardTitle);
const dashboardReportsPanel = getPanelCard(dashboardReportsRoute, dashboardReportsTitle);
const dashboardReportsPage = getPageMetadata(dashboardReportsRoute, dashboardReportsTitle);
export const dashboardPageState = {
    route: dashboardRouteName,
    lessonTitle: dashboardTitle,
    href: dashboardPanel.href,
    label: dashboardPanel.label,
    pageTitle: dashboardPage.title,
};
export const dashboardReportsPageState = {
    route: dashboardReportsRoute,
    lessonTitle: dashboardReportsTitle,
    description: dashboardReportsPage.description,
    imagePath: dashboardReportsPanel.imagePath,
};
//# sourceMappingURL=./dashboard.js.map
