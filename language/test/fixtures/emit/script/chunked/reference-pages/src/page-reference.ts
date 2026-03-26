import {
    missingPageDescription,
    missingPageTitle,
    pageDescriptions,
    pageTitles,
} from "./page-metadata";
import { getSectionLabel } from "./page-sections";

/** Read the display title for one docs page slug. */
export function getPageTitle(slug: string) {
    const pageTitle = pageTitles[slug];

    // fall back when the page title is missing
    if (!pageTitle) {
        return missingPageTitle;
    }

    return pageTitle;
}

/** Read the description for one docs page slug. */
export function getPageDescription(slug: string) {
    const pageDescription = pageDescriptions[slug];

    // fall back when the page description is missing
    if (!pageDescription) {
        return missingPageDescription;
    }

    return pageDescription;
}

/** Build the page heading for one docs page slug. */
export function getPageHeading(slug: string) {
    const pageTitle = getPageTitle(slug);

    // preserve the missing title fallback
    if (pageTitle === missingPageTitle) {
        return missingPageTitle;
    }

    return `Docs: ${pageTitle}`;
}

/** Build the navigation label for one docs page slug. */
export function getNavigationLabel(slug: string) {
    const sectionLabel = getSectionLabel(slug);
    const pageTitle = getPageTitle(slug);

    return `${sectionLabel}: ${pageTitle}`;
}

/** Build the display model for one docs reference page. */
export function getReferencePage(slug: string) {
    const pageTitle = getPageTitle(slug);
    const pageHeading = getPageHeading(slug);
    const pageDescription = getPageDescription(slug);
    const navigationLabel = getNavigationLabel(slug);

    return {
        slug,
        pageTitle,
        pageHeading,
        pageDescription,
        navigationLabel,
    };
}
