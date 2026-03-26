import { getPageDescription, getPageTitle } from "./course-routes.js";
import { getPanelHref, getPanelImagePath, getPanelLabel } from "./course-routes.js";
export const panelRoute = "courses-reports";
export const panelLessonTitle = "lesson-reports";
export const panelHeading = "reports-panel";
export const panelHref = getPanelHref(panelRoute);
export const panelLabel = getPanelLabel(panelRoute, panelLessonTitle);
export const panelPageTitle = getPageTitle(panelLessonTitle);
export const panelPageDescription = getPageDescription(panelRoute);
export const panelImagePath = getPanelImagePath(panelRoute);
//# sourceMappingURL=./report-panel.js.map
