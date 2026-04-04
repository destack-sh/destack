import { getPaginationLabel, getPageNumber, getPageSize } from "./pagination";
import { getPanelSectionTitle, getVisibleSectionCount } from "./panel-sections";

/** Build the summary label for one settings panel state. */
export function getPanelSummary(
    section = "sharing",
    totalCount = 128,
    rawPageSize = "40",
) {
    const pageSize = getPageSize(rawPageSize);
    const page = getPageNumber(pageSize === 50 ? "3" : "2");
    const paginationLabel = getPaginationLabel(totalCount, page, pageSize);
    const sectionTitle = getPanelSectionTitle(section);
    const sectionCount = getVisibleSectionCount(section === "members");

    return `${sectionTitle}:${sectionCount}:${paginationLabel}`;
}
