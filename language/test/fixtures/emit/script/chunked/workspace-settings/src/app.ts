import { getPaginationLabel, getPageNumber, getPageSize } from "./pagination";
import { getPanelSummary } from "./panel-summary";
import { getClipDurationLabel } from "./time-code";
import {
    getWorkspacePath,
    getWorkspaceRoute,
    getWorkspaceTitle,
} from "./workspace";

/** The workspace state for the app entry. */
export const appWorkspaceState = {
    title: getWorkspaceTitle(),
    workspaceRoute: getWorkspaceRoute(),
    workspacePath: getWorkspacePath(),
    paginationLabel: getPaginationLabel(
        240,
        getPageNumber("3"),
        getPageSize("50"),
    ),
    clipDuration: getClipDurationLabel(4, 91),
    panelSummary: getPanelSummary("sharing", 240, "50"),
};

/** The lazy settings panel import for the app entry. */
export const settingsPanelPromise = import("./settings-panel");
