import { getPageMetadata } from "./course-metadata.ts";
import { getOverviewRoute, getReportsRoute } from "./course-routes.ts";
import {
    getOverviewLessonTitle,
    getReportsLessonTitle,
} from "./lesson-titles.ts";
import { getPanelCard } from "./panel-links.ts";

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

/** The lazy reports panel import for the app entry. */
export const reportPanelPromise = import("./report-panel");
export { useState as reactState } from "react";
