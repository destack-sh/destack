import { getPageMetadata } from "./course-metadata.ts";
import { getPanelCard } from "./panel-links.ts";

/** The route slug for the reports panel. */
export const panelRoute = "courses-reports";

/** The lesson title slug for the reports panel. */
export const panelLessonTitle = "lesson-reports";

/** The heading slug for the reports panel. */
export const panelHeading = "reports-panel";

const panelCard = getPanelCard(panelRoute, panelLessonTitle);
const panelPage = getPageMetadata(panelRoute, panelLessonTitle);

/** The route href for the reports panel. */
export const panelHref = panelCard.href;

/** The display label for the reports panel. */
export const panelLabel = panelCard.label;

/** The page title for the reports panel. */
export const panelPageTitle = panelPage.title;

/** The page description for the reports panel. */
export const panelPageDescription = panelPage.description;

/** The image path for the reports panel. */
export const panelImagePath = panelCard.imagePath;
