const statusLabels = {
    idle: "Idle",
    syncing: "Syncing",
    ready: "Ready",
};

/** Read the display label for one bridge status. */
export function getStatusLabel(status = "idle") {
    return statusLabels[status] ?? statusLabels.idle;
}

/** Build the display summary for one bridge status. */
export function getStatusSummary(status = "idle", selectionCount = 0) {
    return `${getStatusLabel(status)}:${selectionCount}`;
}
