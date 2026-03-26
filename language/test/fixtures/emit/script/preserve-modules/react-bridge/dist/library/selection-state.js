export const selectionDefaults = {
    status: "idle",
    selectionCount: 0,
};

export function createSelectionState(
    status = selectionDefaults.status,
    selectionCount = selectionDefaults.selectionCount,
) {
    const normalizedSelectionCount = Math.max(selectionCount, 0);

    return {
        status,
        selectionCount: normalizedSelectionCount,
        isDirty: normalizedSelectionCount > 0,
    };
}
//# sourceMappingURL=./selection-state.map
