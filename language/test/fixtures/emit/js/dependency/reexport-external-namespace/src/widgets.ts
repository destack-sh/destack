/** The local widget groups bundled alongside the external namespace export. */
export const localWidgetGroups = [
    "overview",
    "activity",
    "settings",
];

/** Resolve one local widget route. */
export function resolveLocalWidgetRoute(tab = "overview") {
    return `/widgets/${tab}`;
}
