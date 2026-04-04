/** The shared description for the course dashboard pages. */
export const courseDescription =
    "Course routes, lesson pages, and reporting panels.";

/** Format a lesson title for page title display. */
export function getPageTitle(lessonTitle: string) {
    const displayTitle = lessonTitle.replace(/-/g, " ");

    return `${displayTitle} | academy`;
}

/** Build a page description for a course route. */
export function getPageDescription(route: string) {
    return `${courseDescription} Route: ${route}.`;
}

/** Build the metadata model for one course page. */
export function getPageMetadata(route: string, lessonTitle: string) {
    const title = getPageTitle(lessonTitle);
    const description = getPageDescription(route);

    return {
        title,
        description,
    };
}
