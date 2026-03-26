import { readCourseOutlineEntry } from "./outline-entry.ts";

const appOutlineEntry = readCourseOutlineEntry(
    "course/02-routing/05-dynamic-routes.ts",
);

/** The course outline state for the app entry. */
export const appOutlineState = {
    sectionNumber: appOutlineEntry.sectionNumber,
    lessonNumber: appOutlineEntry.lessonNumber,
    lessonKey: appOutlineEntry.lessonKey,
    lessonSlug: appOutlineEntry.lessonSlug,
};
