import { readCourseOutlineEntry } from "./course-outline.js";
const appOutlineEntry = readCourseOutlineEntry(
    "course/02-routing/05-dynamic-routes.ts",
);
export const appOutlineState = {
    sectionNumber: appOutlineEntry.sectionNumber,
    lessonNumber: appOutlineEntry.lessonNumber,
    lessonKey: appOutlineEntry.lessonKey,
    lessonSlug: appOutlineEntry.lessonSlug,
};
//# sourceMappingURL=./app.js.map
