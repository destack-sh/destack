import { getPageMetadata } from "./course-routes.js";
import { getOverviewRoute, getReportsRoute } from "./course-routes.js";
import { getOverviewLessonTitle, getReportsLessonTitle } from "./course-routes.js";
import { getPanelCard } from "./course-routes.js";
const appRoute = getOverviewRoute();
const appLessonTitle = getOverviewLessonTitle();
const appReportsRoute = getReportsRoute();
const appReportsLessonTitle = getReportsLessonTitle();
const appOverviewPanel = getPanelCard(appRoute, appLessonTitle);
const appOverviewPage = getPageMetadata(appRoute, appLessonTitle);
const appReportsPanel = getPanelCard(appReportsRoute, appReportsLessonTitle);
const appReportsPage = getPageMetadata(appReportsRoute, appReportsLessonTitle);
export const appOverviewPageState = {
    route: appRoute,
    lessonTitle: appLessonTitle,
    href: appOverviewPanel.href,
    label: appOverviewPanel.label,
    pageTitle: appOverviewPage.title,
};
export const appReportsPageState = {
    route: appReportsRoute,
    lessonTitle: appReportsLessonTitle,
    description: appReportsPage.description,
    imagePath: appReportsPanel.imagePath,
};
export const reportPanelPromise = import("./report-panel.js");
export { useState as reactState } from "react";
//# sourceMappingURL=./app.js.map
