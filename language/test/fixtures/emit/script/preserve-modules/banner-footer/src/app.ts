import {
    getReleaseCalendarRoute,
    getWidgetFeedRoute,
} from "./navigation/routes.ts";
import {
    getTimelineHeading,
    getTimelineNavigationLabel,
} from "./navigation/labels.ts";

const appTitle = "timeline";
const appSubtitle = "release-calendar";

/** The timeline page state for the preserve modules entry. */
export const timelinePage = {
    title: appTitle,
    subtitle: appSubtitle,
    route: getReleaseCalendarRoute(),
    widgetFeedRoute: getWidgetFeedRoute("widgets"),
    heading: getTimelineHeading(appSubtitle),
    navigationLabel: getTimelineNavigationLabel("calendar"),
};
