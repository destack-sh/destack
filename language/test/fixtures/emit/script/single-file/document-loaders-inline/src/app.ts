import previewTemplate from "./preview-template.html";
import workspaceMarkText from "./workspace-mark.svg" with { type: "text" };
import workspaceMarkUrl from "./workspace-mark.svg" with { type: "file" };

/** The emitted asset URL for the preview shell. */
export const previewAssetUrl = workspaceMarkUrl;

/** The authored panel template text. */
export const previewTemplateText = previewTemplate;

/** The authored SVG markup for the preview shell. */
export const workspaceMarkSource = workspaceMarkText;
