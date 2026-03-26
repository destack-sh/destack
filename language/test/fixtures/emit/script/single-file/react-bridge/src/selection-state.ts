/** The default bridge selection state. */
export const selectionDefaults = {
    status: "idle",
    selectionCount: 0,
};

/** Build the normalized selection state for one bridge view. */
export function createSelectionState(
    status = selectionDefaults.status,
    selectionCount = selectionDefaults.selectionCount,
) {
    // normalize the incoming selection count
    const normalizedSelectionCount = Math.max(selectionCount, 0);

    // expose a small state snapshot
    return {
        status,
        selectionCount: normalizedSelectionCount,
        isDirty: normalizedSelectionCount > 0,
    };
}
