import { getPageDescription, getPageTitle } from "./course-routes.js";
import { getOverviewRoute, getReportsRoute } from "./course-routes.js";
import { getOverviewLessonTitle, getReportsLessonTitle } from "./course-routes.js";
import { getPanelHref, getPanelImagePath, getPanelLabel } from "./course-routes.js";
const appRoute = getOverviewRoute();
const appLessonTitle = getOverviewLessonTitle();
const appReportsRoute = getReportsRoute();
const appReportsLessonTitle = getReportsLessonTitle();
export const appOverviewPageState = {
    route: appRoute,
    lessonTitle: appLessonTitle,
    href: getPanelHref(appRoute),
    label: getPanelLabel(appRoute, appLessonTitle),
    pageTitle: getPageTitle(appLessonTitle),
};
export const appReportsPageState = {
    route: appReportsRoute,
    lessonTitle: appReportsLessonTitle,
    description: getPageDescription(appReportsRoute),
    imagePath: getPanelImagePath(appReportsRoute),
};
export const reportPanelPromise = import("./report-panel.js");
export { useState as reactState } from "react";
//# sourceMappingURL=./app.js.map
