/** The workspace title for the settings fixture. */
export const workspaceTitle = "notes-workspace";

/** The workspace slug for the settings fixture. */
export const workspaceSlug = "notes";

/** The workspace owner slug for the settings fixture. */
export const workspaceOwner = "editor";

/** Read the workspace title. */
export function getWorkspaceTitle() {
    return workspaceTitle;
}

/** Read the workspace route. */
export function getWorkspaceRoute() {
    return `/workspaces/${workspaceSlug}`;
}

/** Read the workspace owner path. */
export function getWorkspacePath(): string {
    return `${workspaceOwner}/${workspaceSlug}`;
}
