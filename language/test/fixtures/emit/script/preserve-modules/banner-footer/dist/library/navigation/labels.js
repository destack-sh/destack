/*! dashboard widgets */
import { getWidgetFeedRoute } from "./routes.js";

const sectionTitles = {
    overview: "Overview",
    calendar: "Calendar",
    widgets: "Widgets",
};

export function getTimelineHeading(workspace = "release-calendar") {
    return `timeline:${workspace}`;
}

export function getTimelineNavigationLabel(tab = "overview") {
    const sectionTitle = sectionTitles[tab] ?? sectionTitles.overview;

    return `${getTimelineHeading()}:${sectionTitle}:${getWidgetFeedRoute(tab)}`;
}
/* end of widget module */
//# sourceMappingURL=./labels.map
