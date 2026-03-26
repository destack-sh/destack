import { Fragment, useEffect, useState } from "react";
import {
    createSelectionState,
    selectionDefaults,
} from "./selection-state.js";
import {
    describeSelectionState,
    getSelectionModeLabel,
    getSelectionStatusLabel,
} from "./selection-summary.js";
import { getStatusLabel, getStatusSummary } from "./status-store.js";
export { Fragment, useEffect, useState };
export {
    createSelectionState,
    describeSelectionState,
    getSelectionModeLabel,
    getSelectionStatusLabel,
    getStatusLabel,
    getStatusSummary,
    selectionDefaults,
};
export function describeReactBridge(
    status = selectionDefaults.status,
    selectionCount = selectionDefaults.selectionCount,
) {
    const selectionState = createSelectionState(status, selectionCount);
    const selectionSummary = describeSelectionState(
        selectionState.status,
        selectionState.selectionCount,
    );
    const dirtyLabel = selectionState.isDirty ? "dirty" : "clean";
    return `${selectionSummary}:${dirtyLabel}`;
}
//# sourceMappingURL=./react-bridge.map
