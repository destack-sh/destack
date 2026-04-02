export * from "external";

var localWidgetGroups = [
    "overview",
    "activity",
    "settings"
];
function resolveLocalWidgetRoute(tab = "overview") {
    return `/widgets/${tab}`;
}

var localWidgetRegistry = {
    groups: localWidgetGroups,
    route: resolveLocalWidgetRoute("overview")
};
function localWidgetSummary(tab = "overview") {
    const widgetGroupCount = localWidgetGroups.length;
    const widgetRoute = resolveLocalWidgetRoute(tab);

    return `${widgetRoute}:${widgetGroupCount}`;
}
export {
    localWidgetRegistry,
    localWidgetSummary,
    resolveLocalWidgetRoute
};
//# sourceMappingURL=./bundle.js.map
