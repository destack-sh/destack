import { getStatusLabel, getStatusSummary } from "./status-store.js";

export function getSelectionModeLabel(selectionCount = 0) {
    if (selectionCount > 0) {
        return "multi-select";
    }

    return "browse";
}

export function describeSelectionState(status = "idle", selectionCount = 0) {
    const statusSummary = getStatusSummary(status, selectionCount);
    const selectionMode = getSelectionModeLabel(selectionCount);

    return `${statusSummary}:${selectionMode}`;
}

export function getSelectionStatusLabel(status = "idle") {
    return getStatusLabel(status);
}
//# sourceMappingURL=./selection-summary.map
