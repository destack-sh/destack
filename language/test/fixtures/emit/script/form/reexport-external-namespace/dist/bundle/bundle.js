export const localWidgetGroups = ["overview", "activity", "settings"];
export function resolveLocalWidgetRoute(tab = "overview") {
    return `/widgets/${tab}`;
}

export const localWidgetRegistry = {
    groups: localWidgetGroups,
    route: resolveLocalWidgetRoute("overview"),
};
export function localWidgetSummary(tab = "overview") {
    const widgetGroupCount = localWidgetGroups.length;
    const widgetRoute = resolveLocalWidgetRoute(tab);
    return `${widgetRoute}:${widgetGroupCount}`;
}

export * from "external";
export { localWidgetRegistry, localWidgetSummary };
export { resolveLocalWidgetRoute };
//# sourceMappingURL=./bundle.js.map
