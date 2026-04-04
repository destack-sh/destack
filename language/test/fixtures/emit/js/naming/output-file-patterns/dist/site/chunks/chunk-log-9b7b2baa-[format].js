export function log(name, value) {
    console.log(`${name}:${value}`);
}

export const dep = { x: 42 };
export const depLabel = "dep";
export const depRoute = "/dep";
export const depSections = ["overview", "settings", "security"];

export function renderOverview(value, label, route) {
    return `${label}:${route}:${value.x}:${depSections.join("|")}`;
}
//# sourceMappingURL=./chunk-log-9b7b2baa-[format].js.map
