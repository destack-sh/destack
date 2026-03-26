import { readCourseOutlineEntry } from "./outline-entry.ts";

const dashboardOutlineEntry = readCourseOutlineEntry(
    "course/03-rendering/02-static-layouts.ts",
);

/** The course outline state for the dashboard entry. */
export const dashboardOutlineState = {
    sectionNumber: dashboardOutlineEntry.sectionNumber,
    lessonNumber: dashboardOutlineEntry.lessonNumber,
    lessonKey: dashboardOutlineEntry.lessonKey,
    lessonSlug: dashboardOutlineEntry.lessonSlug,
};
