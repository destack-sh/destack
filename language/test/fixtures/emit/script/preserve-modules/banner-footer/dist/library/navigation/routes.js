/*! dashboard widgets */
export const timelineRouteSegments = [
    "timeline",
    "release-calendar",
    "widgets",
];

export function getReleaseCalendarRoute() {
    return `/${timelineRouteSegments.join("/")}`;
}

export function getWidgetFeedRoute(tab = "overview") {
    return `${getReleaseCalendarRoute()}?tab=${tab}`;
}
/* end of widget module */
//# sourceMappingURL=./routes.map
