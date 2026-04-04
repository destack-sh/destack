/** The docs section labels keyed by docs page slug. */
export const sectionLabels = {
    configuration: "Setup",
    github: "Providers",
    google: "Providers",
    authentication: "Security",
    architecture: "Reference",
};

/** Read the section label for one docs page slug. */
export function getSectionLabel(slug: string): string {
    const sectionLabel = sectionLabels[slug];

    // fall back when the docs section is unknown
    if (!sectionLabel) {
        return "Overview";
    }

    return sectionLabel;
}
