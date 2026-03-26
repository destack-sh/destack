import { useEffect, useMemo } from "react";

export function createSelectionState(selectedIds, isFocused) {
    return {
        selectedIds,
        isFocused,
    };
}

export function hasSelection(selectionState) {
    return selectionState.selectedIds.length > 0;
}

export function countSelectedItems(selectionState) {
    return selectionState.selectedIds.length;
}

export function summarizeSelection(selectionState) {
    if (!hasSelection(selectionState)) {
        return "No items selected";
    }

    const selectedCount = countSelectedItems(selectionState);

    if (selectedCount === 1) {
        return "1 item selected";
    }

    return `${selectedCount} items selected`;
}

export function createStatusStore(selectionSummary, isFocused) {
    return {
        selectionSummary,
        isFocused,
    };
}

export function useSelectionStatus(selectionState) {
    const selectionSummary = useMemo(() => {
        return summarizeSelection(selectionState);
    }, [selectionState]);

    const statusStore = createStatusStore(selectionSummary, selectionState.isFocused);

    useEffect(() => {
        console.info(statusStore.selectionSummary);
    }, [statusStore]);

    return statusStore;
}
