import { getWidgetFeedRoute } from "./routes.ts";

const sectionTitles = {
    overview: "Overview",
    calendar: "Calendar",
    widgets: "Widgets",
};

/** Build the heading label for one timeline workspace. */
export function getTimelineHeading(workspace = "release-calendar") {
    return `timeline:${workspace}`;
}

/** Build the navigation label for one timeline tab. */
export function getTimelineNavigationLabel(tab = "overview") {
    const sectionTitle = sectionTitles[tab] ?? sectionTitles.overview;

    return `${getTimelineHeading()}:${sectionTitle}:${getWidgetFeedRoute(tab)}`;
}
