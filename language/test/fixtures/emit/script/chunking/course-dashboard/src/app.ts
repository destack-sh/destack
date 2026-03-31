import { getPageMetadata } from "./course-metadata.ts";
import { getOverviewRoute, getReportsRoute } from "./course-routes.ts";
import {
    getOverviewLessonTitle,
    getReportsLessonTitle,
} from "./lesson-titles.ts";
import { getPanelCard } from "./panel-links.ts";
import {
    panelHeading,
    panelImagePath,
    panelPageDescription,
} from "./report-panel.ts";

const appRoute = getOverviewRoute();
const appLessonTitle = getOverviewLessonTitle();
const appReportsRoute = getReportsRoute();
const appReportsLessonTitle = getReportsLessonTitle();
const appOverviewPanel = getPanelCard(appRoute, appLessonTitle);
const appOverviewPage = getPageMetadata(appRoute, appLessonTitle);
const appReportsPanel = getPanelCard(appReportsRoute, appReportsLessonTitle);
const appReportsPage = getPageMetadata(appReportsRoute, appReportsLessonTitle);

/** The overview page state for the app entry. */
export const appOverviewPageState = {
    route: appRoute,
    lessonTitle: appLessonTitle,
    href: appOverviewPanel.href,
    label: appOverviewPanel.label,
    pageTitle: appOverviewPage.title,
};

/** The reports page state for the app entry. */
export const appReportsPageState = {
    route: appReportsRoute,
    lessonTitle: appReportsLessonTitle,
    description: appReportsPage.description,
    imagePath: appReportsPanel.imagePath,
};

/** The reports panel state for the app entry. */
export const reportPanelState = {
    heading: panelHeading,
    description: panelPageDescription,
    imagePath: panelImagePath,
};

export { useState as reactState } from "react";
