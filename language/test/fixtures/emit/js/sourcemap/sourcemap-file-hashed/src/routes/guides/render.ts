import { formatGuidePath } from "./strings.ts";

/** Render one guide summary string. */
export function renderGuideSummary(slug: string, section: string) {
    const guidePath = formatGuidePath(slug);

    return `${guidePath}#${section}`;
}
