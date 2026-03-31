const sectionTitles = {
    sharing: "Sharing",
    notifications: "Notifications",
    members: "Members",
};

/** Read the display title for one settings panel section. */
export function getPanelSectionTitle(section = "sharing") {
    return sectionTitles[section] ?? sectionTitles.sharing;
}

/** Read the number of visible settings sections. */
export function getVisibleSectionCount(hasMembers = false) {
    return hasMembers ? 3 : 2;
}
