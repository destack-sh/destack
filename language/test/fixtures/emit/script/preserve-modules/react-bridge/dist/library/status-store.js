const statusLabels = {
    idle: "Idle",
    syncing: "Syncing",
    ready: "Ready",
};

export function getStatusLabel(status = "idle") {
    return statusLabels[status] ?? statusLabels.idle;
}

export function getStatusSummary(status = "idle", selectionCount = 0) {
    return `${getStatusLabel(status)}:${selectionCount}`;
}
//# sourceMappingURL=./status-store.map
