import { getPageMetadata } from "./course-metadata.ts";
import { getDashboardRoute, getReportsRoute } from "./course-routes.ts";
import {
    getDashboardLessonTitle,
    getReportsLessonTitle,
} from "./lesson-titles.ts";
import { getPanelCard } from "./panel-links.ts";

const dashboardRouteName = getDashboardRoute();
const dashboardTitle = getDashboardLessonTitle();
const dashboardReportsRoute = getReportsRoute();
const dashboardReportsTitle = getReportsLessonTitle();
const dashboardPanel = getPanelCard(dashboardRouteName, dashboardTitle);
const dashboardPage = getPageMetadata(dashboardRouteName, dashboardTitle);
const dashboardReportsPanel = getPanelCard(
    dashboardReportsRoute,
    dashboardReportsTitle,
);
const dashboardReportsPage = getPageMetadata(
    dashboardReportsRoute,
    dashboardReportsTitle,
);

/** The primary page state for the dashboard entry. */
export const dashboardPageState = {
    route: dashboardRouteName,
    lessonTitle: dashboardTitle,
    href: dashboardPanel.href,
    label: dashboardPanel.label,
    pageTitle: dashboardPage.title,
};

/** The reports page state for the dashboard entry. */
export const dashboardReportsPageState = {
    route: dashboardReportsRoute,
    lessonTitle: dashboardReportsTitle,
    description: dashboardReportsPage.description,
    imagePath: dashboardReportsPanel.imagePath,
};
