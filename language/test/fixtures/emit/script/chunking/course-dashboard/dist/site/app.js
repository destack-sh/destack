import { getPageMetadata } from "./course-routes.js";
import { getPanelCard } from "./course-routes.js";
export const panelRoute = "courses-reports";
export const panelLessonTitle = "lesson-reports";
export const panelHeading = "reports-panel";
const panelCard = getPanelCard(panelRoute, panelLessonTitle);
const panelPage = getPageMetadata(panelRoute, panelLessonTitle);
export const panelHref = panelCard.href;
export const panelLabel = panelCard.label;
export const panelPageTitle = panelPage.title;
export const panelPageDescription = panelPage.description;
export const panelImagePath = panelCard.imagePath;

import { getOverviewRoute, getReportsRoute } from "./course-routes.js";
import { getOverviewLessonTitle, getReportsLessonTitle } from "./course-routes.js";
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
export const reportPanelState = {
    heading: panelHeading,
    description: panelPageDescription,
    imagePath: panelImagePath,
};
export { useState as reactState } from "react";
//# sourceMappingURL=./app.js.map
