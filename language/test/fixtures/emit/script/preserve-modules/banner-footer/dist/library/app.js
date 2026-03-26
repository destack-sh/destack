/*! dashboard widgets */
import { getReleaseCalendarRoute, getWidgetFeedRoute } from "./navigation/routes.js";
import {
    getTimelineHeading,
    getTimelineNavigationLabel,
} from "./navigation/labels.js";
const appTitle = "timeline";
const appSubtitle = "release-calendar";
export const timelinePage = {
    title: appTitle,
    subtitle: appSubtitle,
    route: getReleaseCalendarRoute(),
    widgetFeedRoute: getWidgetFeedRoute("widgets"),
    heading: getTimelineHeading(appSubtitle),
    navigationLabel: getTimelineNavigationLabel("calendar"),
};
/* end of widget module */
//# sourceMappingURL=./app.map
