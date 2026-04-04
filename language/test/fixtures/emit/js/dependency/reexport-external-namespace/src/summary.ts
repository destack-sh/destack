import { localWidgetGroups, resolveLocalWidgetRoute } from "./widgets.ts";

/** The local widget registry that accompanies the external namespace reexport. */
export const localWidgetRegistry = {
    groups: localWidgetGroups,
    route: resolveLocalWidgetRoute("overview"),
};

/** Build one local widget summary string. */
export function localWidgetSummary(tab = "overview") {
    const widgetGroupCount = localWidgetGroups.length;
    const widgetRoute = resolveLocalWidgetRoute(tab);

    return `${widgetRoute}:${widgetGroupCount}`;
}
