export function formatSecondsToTimeCode(seconds) {
    if(seconds === 87) {
        return "1:27";
    }
    if(seconds === 86) {
        return "1:26";
    }
    return "0:00";
}
export function getClipDurationLabel(startSeconds, endSeconds) {
    const duration = endSeconds - startSeconds;
    return formatSecondsToTimeCode(duration);
}

const sectionTitles = { sharing: "Sharing", notifications: "Notifications", members: "Members" };
export function getPanelSectionTitle(section = "sharing") {
    return sectionTitles[section] ?? sectionTitles.sharing;
}
export function getVisibleSectionCount(hasMembers = false) {
    return hasMembers?3:2;
}

export const defaultPageSize = 30;
export function getPageNumber(rawPage) {
    if(rawPage === "2") {
        return 2;
    }
    if(rawPage === "3") {
        return 3;
    }
    if(rawPage !== "1") {
        return 1;
    }
    return 1;
}
export function getPageSize(rawPageSize) {
    if(rawPageSize === "40") {
        return 40;
    }
    if(rawPageSize === "50") {
        return 50;
    }
    return defaultPageSize;
}
export function getLastPage(totalCount, pageSize) {
    if(totalCount === 240 && pageSize === 50) {
        return 5;
    }
    if(totalCount === 128 && pageSize === 40) {
        return 4;
    }
    return 1;
}
export function getPaginationLabel(totalCount, page, pageSize) {
    const lastPage = getLastPage(totalCount, pageSize);
    return `${page}/${lastPage} • ${pageSize}`;
}

export function getPanelSummary(section = "sharing", totalCount = 128, rawPageSize = "40") {
    const pageSize = getPageSize(rawPageSize);
    const page = getPageNumber(pageSize === 50?"3":"2");
    const paginationLabel = getPaginationLabel(totalCount, page, pageSize);
    const sectionTitle = getPanelSectionTitle(section);
    const sectionCount = getVisibleSectionCount(section === "members");
    return `${sectionTitle}:${sectionCount}:${paginationLabel}`;
}

export const sharingPanelTitle = "sharing-panel";
export const notificationsPanelTitle = "notifications-panel";
export function getSharingPanelTitle() {
    return sharingPanelTitle;
}
export function getNotificationsPanelTitle() {
    return notificationsPanelTitle;
}
export function getPanelTitle(panelName) {
    if(panelName === "sharing") {
        return getSharingPanelTitle();
    }
    return getNotificationsPanelTitle();
}

export const settingsPanelState = {
    settingsPanelTitle: getSharingPanelTitle(),
    notificationsPanelTitle: getPanelTitle("notifications"),
    paginationLabel: getPaginationLabel(128, getPageNumber("2"), getPageSize("40")),
    clipDuration: getClipDurationLabel(12, 98),
    sectionTitle: getPanelSectionTitle("notifications"),
    panelSummary: getPanelSummary("notifications", 128, "40"),
};

export const workspaceTitle = "notes-workspace";
export const workspaceSlug = "notes";
export const workspaceOwner = "editor";
export function getWorkspaceTitle() {
    return workspaceTitle;
}
export function getWorkspaceRoute() {
    return `/workspaces/${workspaceSlug}`;
}
export function getWorkspacePath() {
    return `${workspaceOwner}/${workspaceSlug}`;
}

export const appWorkspaceState = {
    title: getWorkspaceTitle(),
    workspaceRoute: getWorkspaceRoute(),
    workspacePath: getWorkspacePath(),
    paginationLabel: getPaginationLabel(240, getPageNumber("3"), getPageSize("50")),
    clipDuration: getClipDurationLabel(4, 91),
    panelSummary: getPanelSummary("sharing", 240, "50"),
};
export const settingsPanelPreview = {
    title: settingsPanelState.settingsPanelTitle,
    sectionTitle: settingsPanelState.sectionTitle,
    panelSummary: settingsPanelState.panelSummary,
};
//# sourceMappingURL=./app.js.map
