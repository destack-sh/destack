export const sharingPanelTitle = "sharing-panel";
export const notificationsPanelTitle = "notifications-panel";
export function getSharingPanelTitle() {
    return sharingPanelTitle;
}
export function getNotificationsPanelTitle() {
    return notificationsPanelTitle;
}
export function getPanelTitle(panelName) {
    if (panelName === "sharing") {
        return getSharingPanelTitle();
    }
    return getNotificationsPanelTitle();
}

import { getPaginationLabel, getPageNumber, getPageSize } from "./app.js";
import { getClipDurationLabel, getPanelSectionTitle, getPanelSummary } from "./app.js";
export const settingsPanelState = {
    settingsPanelTitle: getSharingPanelTitle(),
    notificationsPanelTitle: getPanelTitle("notifications"),
    paginationLabel: getPaginationLabel(128, getPageNumber("2"), getPageSize("40")),
    clipDuration: getClipDurationLabel(12, 98),
    sectionTitle: getPanelSectionTitle("notifications"),
    panelSummary: getPanelSummary("notifications", 128, "40"),
};
//# sourceMappingURL=./settings-panel.js.map
