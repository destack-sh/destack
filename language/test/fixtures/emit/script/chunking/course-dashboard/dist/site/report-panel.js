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
//# sourceMappingURL=./report-panel.js.map
