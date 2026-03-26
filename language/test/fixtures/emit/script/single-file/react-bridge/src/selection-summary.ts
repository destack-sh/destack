import { getStatusLabel, getStatusSummary } from "./status-store.ts";

/** Read the selection mode label for one selection count. */
export function getSelectionModeLabel(selectionCount = 0) {
    // switch into the multi select path
    if (selectionCount > 0) {
        return "multi-select";
    }

    return "browse";
}

/** Build the display summary for one selection state. */
export function describeSelectionState(status = "idle", selectionCount = 0) {
    // gather the status fields
    const statusSummary = getStatusSummary(status, selectionCount);
    const selectionMode = getSelectionModeLabel(selectionCount);

    return `${statusSummary}:${selectionMode}`;
}

/** Read the display label for one selection status. */
export function getSelectionStatusLabel(status = "idle") {
    return getStatusLabel(status);
}
