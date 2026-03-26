/** The route segments for the timeline workspace routes. */
export const timelineRouteSegments = [
    "timeline",
    "release-calendar",
    "widgets",
];

/** Build the release calendar route for the timeline workspace. */
export function getReleaseCalendarRoute() {
    return `/${timelineRouteSegments.join("/")}`;
}

/** Build the widget feed route for one timeline tab. */
export function getWidgetFeedRoute(tab = "overview") {
    return `${getReleaseCalendarRoute()}?tab=${tab}`;
}
