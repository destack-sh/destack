import { getPaginationLabel, getPageNumber, getPageSize } from "./pagination";
import { getPanelSectionTitle } from "./panel-sections";
import { getPanelSummary } from "./panel-summary";
import { getPanelTitle, getSharingPanelTitle } from "./preferences";
import { getClipDurationLabel } from "./time-code";

/** The settings panel state for the workspace settings chunk. */
export const settingsPanelState = {
    settingsPanelTitle: getSharingPanelTitle(),
    notificationsPanelTitle: getPanelTitle("notifications"),
    paginationLabel: getPaginationLabel(
        128,
        getPageNumber("2"),
        getPageSize("40"),
    ),
    clipDuration: getClipDurationLabel(12, 98),
    sectionTitle: getPanelSectionTitle("notifications"),
    panelSummary: getPanelSummary("notifications", 128, "40"),
};
