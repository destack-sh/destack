import { getPageDescription, getPageTitle } from "./course-routes.js";
import { getDashboardRoute, getReportsRoute } from "./course-routes.js";
import { getDashboardLessonTitle, getReportsLessonTitle } from "./course-routes.js";
import { getPanelHref, getPanelImagePath, getPanelLabel } from "./course-routes.js";
const dashboardRouteName = getDashboardRoute();
const dashboardTitle = getDashboardLessonTitle();
const dashboardReportsRoute = getReportsRoute();
const dashboardReportsTitle = getReportsLessonTitle();
export const dashboardPageState = {
    route: dashboardRouteName,
    lessonTitle: dashboardTitle,
    href: getPanelHref(dashboardRouteName),
    label: getPanelLabel(dashboardRouteName, dashboardTitle),
    pageTitle: getPageTitle(dashboardTitle),
};
export const dashboardReportsPageState = {
    route: dashboardReportsRoute,
    lessonTitle: dashboardReportsTitle,
    description: getPageDescription(dashboardReportsRoute),
    imagePath: getPanelImagePath(dashboardReportsRoute),
};
//# sourceMappingURL=./dashboard.js.map
