/** The display title for the sharing panel. */
export const sharingPanelTitle = "sharing-panel";

/** The display title for the notifications panel. */
export const notificationsPanelTitle = "notifications-panel";

/** Read the sharing panel title. */
export function getSharingPanelTitle() {
    return sharingPanelTitle;
}

/** Read the notifications panel title. */
export function getNotificationsPanelTitle() {
    return notificationsPanelTitle;
}

/** Read the title for one settings panel name. */
export function getPanelTitle(panelName: string) {
    if (panelName === "sharing") {
        return getSharingPanelTitle();
    }

    return getNotificationsPanelTitle();
}
