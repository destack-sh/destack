import { getPaginationLabel, getPageNumber, getPageSize } from "./pagination";
import { getPanelSummary } from "./panel-summary";
import { settingsPanelState } from "./settings-panel";
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

/** The settings panel preview state for the app entry. */
export const settingsPanelPreview = {
    title: settingsPanelState.settingsPanelTitle,
    sectionTitle: settingsPanelState.sectionTitle,
    panelSummary: settingsPanelState.panelSummary,
};
