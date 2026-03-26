import { Fragment, useEffect, useState } from "react";
import { createSelectionState, selectionDefaults } from "./selection-state.ts";
import {
    describeSelectionState,
    getSelectionModeLabel,
    getSelectionStatusLabel,
} from "./selection-summary.ts";
import { getStatusLabel, getStatusSummary } from "./status-store.ts";

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

/** Build the display summary for one React bridge state. */
export function describeReactBridge(
    status = selectionDefaults.status,
    selectionCount = selectionDefaults.selectionCount,
) {
    // read the current selection state
    const selectionState = createSelectionState(status, selectionCount);

    // derive a human readable summary
    const selectionSummary = describeSelectionState(
        selectionState.status,
        selectionState.selectionCount,
    );

    // tag whether the state is still pristine
    const dirtyLabel = selectionState.isDirty ? "dirty" : "clean";

    return `${selectionSummary}:${dirtyLabel}`;
}
