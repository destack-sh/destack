import { depSections } from "../dep.ts";

export function renderOverview(
    value: { x: number },
    label: string,
    route: string,
) {
    return `${label}:${route}:${value.x}:${depSections.join("|")}`;
}
