import { sharedNavigationItems } from "./navigation.ts";

/** Format one navigation label using the shared navigation table. */
export function formatNavigationLabel(section = "home") {
    const sharedNavigationPrefix = sharedNavigationItems.join("|");

    return `${section}:${sharedNavigationPrefix}`;
}
