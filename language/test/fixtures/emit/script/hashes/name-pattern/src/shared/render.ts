import { sharedNavigationItems } from "./navigation.ts";

/** Render one navigation summary string. */
export function renderNavigationSummary(section: string, label: string) {
    const sharedNavigationCount = sharedNavigationItems.length;

    return `${section}:${label}:${sharedNavigationCount}`;
}
